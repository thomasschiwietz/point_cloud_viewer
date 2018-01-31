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

use graphic::{GlBuffer, GlProgram, GlVertexArray};
use opengl;
use opengl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use glhelper::{compile_shader, link_program};
use std::str;
use std::mem;
use std::ptr;
use cgmath::{Array, Matrix, Matrix4};

const FRAGMENT_SHADER_FILLED_BOX: &'static str = include_str!("../shaders/filledBox.fs");
const VERTEX_SHADER_FILLED_BOX: &'static str = include_str!("../shaders/filledBox.vs");

pub struct BoxDrawer2<'a>
{
    // filled program, buffer, uniform locations
    filled_program: GlProgram<'a>,
    filled_u_transform: GLint,
    filled_u_model_view_transform: GLint,
    filled_u_color: GLint,
    filled_vertex_array: GlVertexArray<'a>,
    _filled_buffer_position: GlBuffer<'a>,
    _filled_buffer_normals: GlBuffer<'a>,
    _filled_buffer_indices: GlBuffer<'a>,
}

impl<'a> BoxDrawer2<'a> {
    pub fn new(gl: &'a opengl::Gl) -> Self {
        let filled_program = GlProgram::new(gl, VERTEX_SHADER_FILLED_BOX, FRAGMENT_SHADER_FILLED_BOX);  
        let filled_u_transform;
        let filled_u_model_view_transform;
        let filled_u_color;
    
        unsafe {
            gl.UseProgram(filled_program.id);
            filled_u_transform = gl.GetUniformLocation(filled_program.id, c_str!("transform"));
            filled_u_model_view_transform = gl.GetUniformLocation(filled_program.id, c_str!("modelViewTransform"));
            filled_u_color = gl.GetUniformLocation(filled_program.id, c_str!("color"));
        }

        let filled_vertex_array = GlVertexArray::new(gl);
        filled_vertex_array.bind();

        // vertex buffer for filled box
        let _filled_buffer_position = GlBuffer::new_array_buffer(gl);
        _filled_buffer_position.bind();
        let per_face_vertices: [f32; 3*4] = [
            // top
            -1.0, 1.0, 0.0,
            1.0, 1.0, 0.0, 
            1.0, -1.0, 0.0, 
            -1.0, -1.0, 0.0,
        ];
        unsafe {
            gl.BufferData(
                opengl::ARRAY_BUFFER,
                (per_face_vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&per_face_vertices[0]),
                opengl::STATIC_DRAW,
            );
        }

        unsafe{
            let pos_attr = gl.GetAttribLocation(filled_program.id, c_str!("aPos"));
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

        // vertex buffer for filled box: normals
        let _filled_buffer_normals = GlBuffer::new_array_buffer(gl);
        _filled_buffer_normals.bind();
        let per_face_vertex_normals: [f32; 3*4] = [
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
        ];
        unsafe {
            gl.BufferData(
                opengl::ARRAY_BUFFER,
                (per_face_vertex_normals.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&per_face_vertex_normals[0]),
                opengl::STATIC_DRAW,
            );
        }

        unsafe {
            let normal_attr = gl.GetAttribLocation(filled_program.id, c_str!("aNormal"));
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

        // define index buffer for 24 edges of the box
        let _filled_buffer_indices = GlBuffer::new_element_array_buffer(gl);
        _filled_buffer_indices.bind();
        let mut filled_indices: [i32; 6] = [0,0,0,0,0,0];
        // for i in 0..6 {
            let o = 0;//6 * i;
            let v = 0;//4 * i as i32;

            filled_indices[o + 0] = v + 0;
            filled_indices[o + 1] = v + 1;
            filled_indices[o + 2] = v + 2;

            filled_indices[o + 3] = v + 2;
            filled_indices[o + 4] = v + 3;
            filled_indices[o + 5] = v + 0;
        //}
        unsafe {
            gl.BufferData(
                opengl::ELEMENT_ARRAY_BUFFER,
                (filled_indices.len() * mem::size_of::<i32>()) as GLsizeiptr,
                mem::transmute(&filled_indices[0]),
                opengl::STATIC_DRAW,
            );
        }

        BoxDrawer2 {
            filled_program,
            filled_u_transform,
            filled_u_model_view_transform,
            filled_u_color,
            filled_vertex_array,
            _filled_buffer_position,
            _filled_buffer_normals,
            _filled_buffer_indices,
        }        
    }

    pub fn draw_filled(&self, color: &Vec<f32>, world_to_view: &Matrix4<f32>, world_to_gl: &Matrix4<f32>) {
        self.filled_vertex_array.bind();

        unsafe {
            self.filled_program.gl.UseProgram(self.filled_program.id);
            self.filled_program.gl.UniformMatrix4fv(self.filled_u_transform, 1, false as GLboolean, world_to_gl.as_ptr());
            self.filled_program.gl.UniformMatrix4fv(self.filled_u_model_view_transform, 1, false as GLboolean, world_to_view.as_ptr());
            self.filled_program.gl.Uniform4fv(self.filled_u_color, 1, color.as_ptr());
            self.filled_program.gl.DrawElements(opengl::TRIANGLES, 6, opengl::UNSIGNED_INT, ptr::null());
            // self.filled_program.gl.DrawArrays(opengl::TRIANGLES, 0, 2);
        }
    }
}
