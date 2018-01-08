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

use gl;
use glhelper::{compile_shader, link_program};
use graphic::{GlBuffer, GlProgram, GlVertexArray};
use gl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use std::str;
use std::mem;
use std::ptr;
use cgmath::{Array, Matrix, Matrix4};

const FRAGMENT_SHADER_OUTLINED_BOX: &'static str = include_str!("../shaders/outlinedBox.fs");
const VERTEX_SHADER_OUTLINED_BOX: &'static str = include_str!("../shaders/outlinedBox.vs");

pub struct BoxDrawer
{
    // outlines program, buffer, uniform locations
    outlines_program: GlProgram,
    outlines_u_transform: GLint,
    outlines_u_color: GLint,
    outlines_vertex_array: GlVertexArray,
    _outlines_buffer_position: GlBuffer,
    _outlines_buffer_indices: GlBuffer,

    // filled program, buffer, uniform locations
    filled_vertex_array: GlVertexArray,
    _filled_buffer_position: GlBuffer,
}

impl BoxDrawer {
    pub fn new() -> Self {
        let outlines_program = GlProgram::new(VERTEX_SHADER_OUTLINED_BOX, FRAGMENT_SHADER_OUTLINED_BOX);  
        let outlines_u_transform;
        let outlines_u_color;
    
        unsafe {
            gl::UseProgram(outlines_program.id);
            outlines_u_transform = gl::GetUniformLocation(outlines_program.id, c_str!("transform"));
            outlines_u_color = gl::GetUniformLocation(outlines_program.id, c_str!("color"));
        }

        let outlines_vertex_array = GlVertexArray::new();
        outlines_vertex_array.bind();

        // vertex buffer: define 8 vertices of the box
        let _outlines_buffer_position = GlBuffer::new();
        _outlines_buffer_position.bind(gl::ARRAY_BUFFER);
        let vertices: [f32; 3*8] = [
            -1.0, -1.0, 1.0,
            1.0, -1.0, 1.0,
            1.0,  1.0, 1.0,
            -1.0,  1.0, 1.0,
            -1.0, -1.0, -1.0,
            1.0, -1.0, -1.0,
            1.0,  1.0, -1.0,
            -1.0,  1.0, -1.0,
        ];
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&vertices[0]),
                gl::STATIC_DRAW,
            );
        }

        // define index buffer for 24 edges of the box
        let _outlines_buffer_indices = GlBuffer::new();
        _outlines_buffer_indices.bind(gl::ELEMENT_ARRAY_BUFFER);
        let indices: [i32; 24] = [
            0,1, 1,2, 2,3, 3,0,		// front
		    4,5, 5,6, 6,7, 7,4,		// back
		    1,5, 6,2,				// right
		    4,0, 3,7,				// left
        ];
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<i32>()) as GLsizeiptr,
                mem::transmute(&indices[0]),
                gl::STATIC_DRAW,
            );
        }

        unsafe{
            let pos_attr = gl::GetAttribLocation(outlines_program.id, c_str!("aPos"));
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as GLuint,
                3,
                gl::FLOAT,
                gl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        let filled_vertex_array = GlVertexArray::new();
        filled_vertex_array.bind();

        // vertex buffer: define 8 vertices of the box
        let _filled_buffer_position = GlBuffer::new();
        _filled_buffer_position.bind(gl::ARRAY_BUFFER);
        let per_face_vertices: [f32; 3*4*6] = [
	        // front
            -1.0, -1.0, 1.0,
            1.0, -1.0, 1.0,
            1.0, 1.0, 1.0,
            -1.0, 1.0, 1.0,
            // back
            1.0, -1.0, -1.0,
            -1.0, -1.0, -1.0,
            -1.0, 1.0, -1.0,
            1.0, 1.0, -1.0,
            // right
            1.0, -1.0, 1.0,
            1.0, -1.0, -1.0,
            1.0, 1.0, -1.0,
            1.0, 1.0, 1.0,
            // left
            -1.0, -1.0, -1.0,
            -1.0, -1.0, 1.0,
            -1.0, 1.0, 1.0,
            -1.0, 1.0, -1.0,
            // top
            -1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
            1.0, 1.0, -1.0,
            -1.0, 1.0, -1.0,
            // bottom
            1.0, -1.0, 1.0,
            -1.0, -1.0, 1.0,
            -1.0, -1.0, -1.0,
            1.0, -1.0, -1.0,
        ];
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (per_face_vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&per_face_vertices[0]),
                gl::STATIC_DRAW,
            );
        }

        BoxDrawer {
            outlines_program,
            outlines_u_transform,
            outlines_u_color,
            outlines_vertex_array,
            _outlines_buffer_position,
            _outlines_buffer_indices,
            filled_vertex_array,
            _filled_buffer_position,
        }        
    }

    pub fn update_transform(&self, matrix: &Matrix4<f32>) {
        unsafe {
            gl::UseProgram(self.outlines_program.id);
            gl::UniformMatrix4fv(self.outlines_u_transform, 1, false as GLboolean, matrix.as_ptr());
        }
    }

    pub fn update_color(&self, color: &Vec<f32>) {
        unsafe {
            gl::UseProgram(self.outlines_program.id);
            gl::Uniform4fv(self.outlines_u_color, 1, color.as_ptr());
        }
    }

    pub fn draw_outlines(&self) {
        self.outlines_vertex_array.bind();

        unsafe {
            gl::UseProgram(self.outlines_program.id);
            gl::DrawElements(gl::LINES, 24, gl::UNSIGNED_INT, ptr::null());
        }
    }
}
