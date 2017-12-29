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
use graphic::GlProgram;
use gl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use std::str;
use std::mem;
use std::ptr;
use cgmath::{Array, Matrix, Matrix4};
use quad_buffer::QuadBuffer;

const FRAGMENT_SHADER_QUAD: &'static str = include_str!("../shaders/quad_drawer.fs");
const VERTEX_SHADER_QUAD: &'static str = include_str!("../shaders/quad_drawer.vs");

pub struct QuadDrawer
{
    quad_buffer: QuadBuffer,

    program: GlProgram,
    u_texture_id: GLint,
}

impl QuadDrawer {
    pub fn new() -> Self {
        let quad_buffer = QuadBuffer::new();

        let program = GlProgram::new(VERTEX_SHADER_QUAD, FRAGMENT_SHADER_QUAD);  
        let u_texture_id;
        unsafe {
            gl::UseProgram(program.id);
            u_texture_id = gl::GetUniformLocation(program.id, c_str!("aTex"));
        }

        quad_buffer.vertex_array.bind();
        unsafe{
            let pos_attr = gl::GetAttribLocation(program.id, c_str!("aPos"));
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE,
                2 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        QuadDrawer {
            quad_buffer,
            program,
            u_texture_id,
        }        
    }

    pub fn draw(&self, texture_id: GLuint) {

        unsafe {
            gl::UseProgram(self.program.id);
            //gl::Disable(gl::DEPTH);           // causes opengl error?

            // bind texture to unit 0
            gl::Uniform1i(self.u_texture_id, 0);
            gl::ActiveTexture(gl::TEXTURE0 + 0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            self.quad_buffer.draw();

            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}
