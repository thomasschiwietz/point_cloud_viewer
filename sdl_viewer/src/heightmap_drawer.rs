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

use cgmath::{InnerSpace, Matrix, Matrix4, Vector3};
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

const HM_FRAGMENT_SHADER: &str = include_str!("../shaders/heightmap.fs");
const HM_VERTEX_SHADER: &str = include_str!("../shaders/heightmap.vs");

pub struct HeightMapDrawer<'a> {
    program: GlProgram<'a>,

    // Uniforms locations.
    u_transform: GLint,
    u_model_view_transform: GLint,
    u_color: GLint,

    // Vertex array and buffers
    vertex_array: GlVertexArray<'a>,
    buffer_position: GlBuffer<'a>,
    buffer_normal: GlBuffer<'a>,
    num_indices: usize,
}

impl<'a> HeightMapDrawer<'a> {
    pub fn new(gl: &'a opengl::Gl) -> Self {
        let program =
            GlProgram::new(gl, HM_VERTEX_SHADER, HM_FRAGMENT_SHADER);
        let u_transform;
        let u_model_view_transform;
        let u_color;

        unsafe {
            gl.UseProgram(program.id);
            u_transform = gl.GetUniformLocation(program.id, c_str!("transform"));
            u_model_view_transform = gl.GetUniformLocation(program.id, c_str!("modelViewTransform"));
            u_color = gl.GetUniformLocation(program.id, c_str!("color"));
        }

        let vertex_array = GlVertexArray::new(gl);
        vertex_array.bind();

        let buffer_position = GlBuffer::new_array_buffer(gl);

        let buffer_normal = GlBuffer::new_array_buffer(gl);

        unsafe {
            buffer_position.bind();
            let pos_attr = gl.GetAttribLocation(program.id, c_str!("aPos"));
            println!("pos {}", pos_attr);
            gl.EnableVertexAttribArray(pos_attr as GLuint);
            gl.VertexAttribPointer(
                pos_attr as GLuint,
                3,
                opengl::FLOAT,
                opengl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        unsafe {
            buffer_normal.bind();
            let normal_attr = gl.GetAttribLocation(program.id, c_str!("aNormal"));
            println!("normal_attr {}", normal_attr);
            gl.EnableVertexAttribArray(normal_attr as GLuint);
            gl.VertexAttribPointer(
                normal_attr as GLuint,
                3,
                opengl::FLOAT,
                opengl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        let num_indices = 0;

        HeightMapDrawer {
            program,
            u_transform,
            u_model_view_transform,
            u_color,
            vertex_array,
            buffer_position,
            buffer_normal,
            num_indices,
        }
    }

    fn linear_index(x: i32, y: i32, size: i32) -> usize {
        (x + y * size) as usize
    }

    fn compute_triangle_normal(v0: &Vector3<f32>, v1: &Vector3<f32>, v2: &Vector3<f32>) -> Vector3<f32> {
        let t0 = v2 - v0;
        let t1 = v1 - v0;
        let n = t0.cross(t1);
        -n.normalize()
    }

    pub fn load_proto(&mut self, height_map_file_name: String) {
        println!("loading height map from {}", height_map_file_name);
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
                    origin_y + (y as f32 * resolution_m),
                    // f32::sin(x as f32 / (0.25 * size as f32) * y as f32 / (0.25 * size as f32)) + 50.
                    ground_map_proto.z[i] as f32 + 50.,
                );
                grid_vertices.push(v);
                i += 1;
            }
        }
        //println!("{:?}", grid_vertices);

        // let mut vertex_normals = Vec::new();
        // for y in 0..size {
        //     for x in 0..size {
        //         vertex_normals[HeightMapDrawer::linear_index(x, y, size)] = Vec::new();
        //     }
        // }

        // compute triangle list
        let mut triangle_vertices = Vec::new();
        let mut triangle_normals = Vec::new();
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
                let normal0 = HeightMapDrawer::compute_triangle_normal(&v00, &v10, &v11);
                triangle_normals.push(normal0);
                triangle_normals.push(normal0);
                triangle_normals.push(normal0);
                //vertex_normals[HeightMapDrawer::linear_index(x, y, size)].push(normal0);
                //vertex_normals[HeightMapDrawer::linear_index(x+1, y, size)].push(normal0);
                //vertex_normals[HeightMapDrawer::linear_index(x+1, y+1, size)].push(normal0);

                // upper triangle
                triangle_vertices.push(v00);
                triangle_vertices.push(v11);
                triangle_vertices.push(v01);
                let normal1 = HeightMapDrawer::compute_triangle_normal(&v00, &v11, &v01);
                triangle_normals.push(normal1);
                triangle_normals.push(normal1);
                triangle_normals.push(normal1);
                //vertex_normals[HeightMapDrawer::linear_index(x, y, size)].push(normal1);
                //vertex_normals[HeightMapDrawer::linear_index(x+1, y+1, size)].push(normal1);
                //vertex_normals[HeightMapDrawer::linear_index(x, y+1, size)].push(normal1);
            }
        }
        self.num_indices = triangle_vertices.len();
        //println!("{:?}", self.triangle_vertices);

        // compute vertex normals
        // let vertex_normals = triangle_normals.clone();
        // for y in 1..size-1 {
        //     for x in 1..size-1 {
        //         let vn = 
        //         ;
        //         vertex_normals[HeightMapDrawer::linear_index(x, y, size)] = vn;
        //     }
        // }

        println!("number of triangles {} / {}", self.num_indices / 3, triangle_normals.len() / 3);

        self.vertex_array.bind();

        self.buffer_position.bind();
        unsafe {
            self.program.gl.BufferData(
                opengl::ARRAY_BUFFER,
                (triangle_vertices.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                triangle_vertices.as_ptr() as *const c_void,
                opengl::STATIC_DRAW,
            );
        }

        self.buffer_normal.bind();
        unsafe {
            self.program.gl.BufferData(
                opengl::ARRAY_BUFFER,
                (triangle_normals.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                triangle_normals.as_ptr() as *const c_void,
                opengl::STATIC_DRAW,
            );
        }

    }

    pub fn draw(&self, color: &Vec<f32>, world_to_view: &Matrix4<f32>, world_to_gl: &Matrix4<f32>) {
        self.vertex_array.bind();

        unsafe {
            self.program.gl.UseProgram(self.program.id);
            self.program.gl.UniformMatrix4fv(self.u_transform, 1, false as GLboolean, world_to_gl.as_ptr());
            self.program.gl.UniformMatrix4fv(self.u_model_view_transform, 1, false as GLboolean, world_to_view.as_ptr());
            self.program.gl.Uniform4fv(self.u_color, 1, color.as_ptr());
            self.program.gl.DrawArrays(opengl::TRIANGLES, 0, self.num_indices as i32);
        }
    }
}
