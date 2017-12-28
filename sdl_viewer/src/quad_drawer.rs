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

const FRAGMENT_SHADER_QUAD: &'static str = include_str!("../shaders/quad_drawer.fs");
const VERTEX_SHADER_QUAD: &'static str = include_str!("../shaders/quad_drawer.vs");

pub struct QuadDrawer
{
    program: GlProgram,

    u_texture_id: GLint,

    // vertex array and buffers
    vertex_array: GlVertexArray,
    _buffer_position: GlBuffer,
    _buffer_indices: GlBuffer,
}

impl QuadDrawer {
    pub fn new() -> Self {
        let program = GlProgram::new(VERTEX_SHADER_QUAD, FRAGMENT_SHADER_QUAD);  
        let u_texture_id;
        unsafe {
            gl::UseProgram(program.id);
            u_texture_id = gl::GetUniformLocation(program.id, c_str!("aTex"));
            println!("textureid {}", u_texture_id);
        }

        let vertex_array = GlVertexArray::new();
        vertex_array.bind();

        // vertex buffer: define 4 vertices of the quad
        let _buffer_position = GlBuffer::new();
        _buffer_position.bind(gl::ARRAY_BUFFER);
        let vertices: [[f32;3]; 4] = [
            [-1.0, -1.0, 0.0],
            [ 1.0, -1.0, 0.0],
            [ 1.0,  1.0, 0.0],
            [-1.0,  1.0, 0.0],
        ];
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&vertices[0]),
                gl::STATIC_DRAW,
            );
        }

        // define index buffer for 24 edges of the box
        let _buffer_indices = GlBuffer::new();
        _buffer_indices.bind(gl::ELEMENT_ARRAY_BUFFER);
        let indices: [i32; 2*3] = [
            0,1,2,
            0,2,3,
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
            let pos_attr = gl::GetAttribLocation(program.id, c_str!("aPos"));
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
        unsafe {
            println!("quad: gl error {}", gl::GetError());
        }
        QuadDrawer {
            program,
            u_texture_id,
            vertex_array,
            _buffer_position,
            _buffer_indices
        }        
    }

    pub fn draw(&self, texture_id: GLuint) {
        self.vertex_array.bind();

        unsafe {
            gl::UseProgram(self.program.id);
            gl::Disable(gl::DEPTH);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        }
    }
}
