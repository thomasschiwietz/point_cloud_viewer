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
use graphic::GlProgram;
use gl::types::{GLboolean, GLint, GLuint};
use std::str;
use std::mem;
use std::ptr;
use quad_buffer::QuadBuffer;
use cgmath::{Matrix, Matrix4};

const FRAGMENT_SHADER_QUAD: &'static str = include_str!("../shaders/image_drawer.fs");
const VERTEX_SHADER_QUAD: &'static str = include_str!("../shaders/image_drawer.vs");

pub struct ImageDrawer
{
    quad_buffer: QuadBuffer,

    program: GlProgram,
    u_texture_id: GLint,
    u_matrix: GLint,
}

impl ImageDrawer {
    pub fn new() -> Self {
        let quad_buffer = QuadBuffer::new();

        let program = GlProgram::new(VERTEX_SHADER_QUAD, FRAGMENT_SHADER_QUAD);  
        let u_texture_id;
        let u_matrix;
        unsafe {
            gl::UseProgram(program.id);
            u_texture_id = gl::GetUniformLocation(program.id, c_str!("aTex"));
            u_matrix = gl::GetUniformLocation(program.id, c_str!("matrix"));
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

        ImageDrawer {
            quad_buffer,
            program,
            u_texture_id,
            u_matrix,
        }        
    }

    pub fn draw(&self, texture_id: GLuint, matrix: &Matrix4<f32>) {

        unsafe {
            gl::UseProgram(self.program.id);
            gl::Disable(gl::DEPTH_TEST);
            gl::DepthMask(gl::FALSE);

            // bind texture to unit 0
            gl::Uniform1i(self.u_texture_id, 0);
            gl::ActiveTexture(gl::TEXTURE0 + 0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            gl::UniformMatrix4fv(self.u_matrix, 1, false as GLboolean, matrix.as_ptr());

            self.quad_buffer.draw();

            gl::BindTexture(gl::TEXTURE_2D, 0);

            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::DEPTH_TEST);
        }
    }
}
