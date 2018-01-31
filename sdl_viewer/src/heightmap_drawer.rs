// Copyright 2016 The Cartographer Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cgmath::{Matrix, Matrix4, Vector3};
use color::Color;
use graphic::{GlBuffer, GlProgram, GlVertexArray};
use opengl;
use opengl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use std::mem;
use std::os::raw::c_void;
use std::ptr;

use protobuf;
use proto;

use std::fs::File;
use std::io::{Cursor, Read};

const FRAGMENT_SHADER_OUTLINED_BOX: &str = include_str!("../shaders/heightmap.fs");
const VERTEX_SHADER_OUTLINED_BOX: &str = include_str!("../shaders/heightmap.vs");

pub struct HeightMapDrawer<'a> {
    program: GlProgram<'a>,

    // Uniforms locations.
    u_transform: GLint,
    u_color: GLint,

    // Vertex array and buffers
    vertex_array: GlVertexArray<'a>,
    buffer_position: GlBuffer<'a>,
    // buffer_normal: GlBuffer<'a>,
}

impl<'a> HeightMapDrawer<'a> {
    pub fn new(gl: &'a opengl::Gl) -> Self {
        let program =
            GlProgram::new(gl, VERTEX_SHADER_OUTLINED_BOX, FRAGMENT_SHADER_OUTLINED_BOX);
        let u_transform;
        let u_color;

        unsafe {
            gl.UseProgram(program.id);
            u_transform = gl.GetUniformLocation(program.id, c_str!("transform"));
            u_color = gl.GetUniformLocation(program.id, c_str!("color"));
        }

        let vertex_array = GlVertexArray::new(gl);
        vertex_array.bind();

        let buffer_position = GlBuffer::new_array_buffer(gl);
        // let buffer_normal = GlBuffer::new_array_buffer(gl);

        unsafe {
            let pos_attr = gl.GetAttribLocation(program.id, c_str!("position"));
            gl.EnableVertexAttribArray(pos_attr as GLuint);
            gl.VertexAttribPointer(
                pos_attr as GLuint,
                3,
                opengl::FLOAT,
                opengl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );

            // let normal_attr = gl.GetAttribLocation(program.id, c_str!("normal"));
            // gl.EnableVertexAttribArray(normal_attr as GLuint);
            // gl.VertexAttribPointer(
            //     pos_attr as GLuint,
            //     3,
            //     opengl::FLOAT,
            //     opengl::FALSE,
            //     3 * mem::size_of::<f32>() as i32,
            //     ptr::null(),
            // );
        }
        HeightMapDrawer {
            program,
            u_transform,
            u_color,
            vertex_array,
            buffer_position,
            //buffer_normal,
        }
    }

    fn linear_index(x: i32, y: i32, size: i32) -> usize {
        (x + y * size) as usize
    }

    pub fn load_proto(&self, gl: &'a opengl::Gl, height_map_file_name: String) {
        // read proto
        let ground_map_proto = {
            let mut data = Vec::new();
            File::open(&height_map_file_name).unwrap().read_to_end(&mut data).unwrap();
            protobuf::parse_from_reader::<proto::GroundMap>(&mut Cursor::new(data)).unwrap()
        };
        let size = ground_map_proto.size;
        let resolution_m = ground_map_proto.resolution_m as f32;
        let origin_x = ground_map_proto.origin_x as f32;
        let origin_y = ground_map_proto.origin_y as f32;

        // compute grid vertices
        let mut grid_vertices = Vec::new();
        let mut i = 0;
        for y in 0..size {
            for x in 0..size {
                let v = Vector3::new(
                    origin_x + (x as f32 * resolution_m),
                    ground_map_proto.z[i] as f32,
                    origin_y + (y as f32 * resolution_m),
                );
                grid_vertices.push(v);
                i += 1;
            }
        }

        // compute triangle list
        let mut triangle_vertices = Vec::new();
        for y in 0..size-1 {
            for x in 0..size-1 {
                // get vertices
                let v00 = grid_vertices[HeightMapDrawer::linear_index(x, y, size)];
                let v10 = grid_vertices[HeightMapDrawer::linear_index(x+1, y, size)];
                let v01 = grid_vertices[HeightMapDrawer::linear_index(x, y+1, size)];
                let v11 = grid_vertices[HeightMapDrawer::linear_index(x+1, y+1, size)];

                // lower triangle
                triangle_vertices.push(v00);
                triangle_vertices.push(v10);
                triangle_vertices.push(v11);

                // upper triangle
                triangle_vertices.push(v00);
                triangle_vertices.push(v11);
                triangle_vertices.push(v01);
            }
        }

        self.buffer_position.bind();
        unsafe {
            gl.BufferData(
                opengl::ARRAY_BUFFER,
                (triangle_vertices.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                triangle_vertices.as_ptr() as *const [f32; 3] as *const c_void,
                opengl::STATIC_DRAW,
            );
        }
    }

    pub fn draw(&self, world_to_gl: &Matrix4<f32>, color: &Color) {
        self.vertex_array.bind();

        unsafe {
            self.program.gl.UseProgram(self.program.id);
            self.program.gl.UniformMatrix4fv(
                self.u_transform,
                1,
                false as GLboolean,
                world_to_gl.as_ptr(),
            );
            self.program.gl.Uniform4f(
                self.u_color,
                color.red,
                color.green,
                color.blue,
                color.alpha,
            );
            self.program.gl.DrawElements(
                opengl::LINES,
                24,
                opengl::UNSIGNED_INT,
                ptr::null(),
            );
        }
    }
}
