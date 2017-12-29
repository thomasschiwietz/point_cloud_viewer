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
use graphic::{GlProgram, GlFramebuffer, TextureType};
use gl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use std::str;
use std::mem;
use std::ptr;
use quad_buffer::QuadBuffer;

const FRAGMENT_SHADER_REDUCE_MAX: &'static str = include_str!("../shaders/reduce_max.fs");
const VERTEX_SHADER_REDUCTION: &'static str = include_str!("../shaders/quad_drawer.vs");

pub struct Reduction
{
    quad_buffer: QuadBuffer,

    frame_buffers: [GlFramebuffer; 2],

    program_max: GlProgram,
    u_max_texture_id: GLint,
    u_max_step: GLint,
}

impl Reduction {
    pub fn new(width: i32, height: i32) -> Self {
        let quad_buffer = QuadBuffer::new();
        let frame_buffers = [
            GlFramebuffer::new(width, height, TextureType::ColorR32F, TextureType::Uninitialized), 
            GlFramebuffer::new(width, height, TextureType::ColorR32F, TextureType::Uninitialized),
        ];

        let program_max = GlProgram::new(VERTEX_SHADER_REDUCTION, FRAGMENT_SHADER_REDUCE_MAX);  
        let u_max_texture_id;
        let u_max_step;
        unsafe {
            gl::UseProgram(program_max.id);
            u_max_texture_id = gl::GetUniformLocation(program_max.id, c_str!("aTex"));
            u_max_step = gl::GetUniformLocation(program_max.id, c_str!("step"));
        }

        quad_buffer.vertex_array.bind();
        unsafe{
            let pos_attr = gl::GetAttribLocation(program_max.id, c_str!("aPos"));
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
        Reduction {
            quad_buffer,
            frame_buffers,
            program_max,
            u_max_texture_id,
            u_max_step,
        }        
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        self.frame_buffers[0].set_size(width, height);
        self.frame_buffers[1].set_size(width, height);
    }

    // return texture_id of result
    pub fn reduce_max(&self, texture_id: GLuint) -> GLuint {
        // texture dimensions of texture_ID and internal frame buffer must match!
        // save current viewport

        let orig_width = self.frame_buffers[0].width;
        let orig_height = self.frame_buffers[0].height;

        let mut dst_width = orig_width / 2;
        let mut dst_height = orig_height / 2;
        let mut src_texture_scale = 1.;

        let mut src_framebuffer = 1;
        let mut dst_framebuffer = 0;

        for i in 0..1 {
            // setup target frame buffer
            self.frame_buffers[dst_framebuffer].bind();
            unsafe {
                gl::ClearColor(1., 1., 1., 1.);
                gl::Clear(gl::COLOR_BUFFER_BIT);

                gl::Viewport(0, 0, dst_width, dst_height);
                gl::Scissor(0, 0, dst_width, dst_height);
                gl::Enable(gl::SCISSOR_TEST);
            }

            let max_step = vec![1. / orig_width as f32, 1. / orig_height as f32, src_texture_scale, 0.];

            unsafe {
                gl::UseProgram(self.program_max.id);
                gl::Uniform4fv(self.u_max_step, 1, max_step.as_ptr());

                // bind texture to texture unit 0
                gl::Uniform1i(self.u_max_texture_id, 0);
                gl::ActiveTexture(gl::TEXTURE0 + 0);

                // source from external texture the first time
                if i == 0 {
                    gl::BindTexture(gl::TEXTURE_2D, texture_id);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, self.frame_buffers[src_framebuffer].color_texture.id);                    
                }

                self.quad_buffer.draw();

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }

            self.frame_buffers[dst_framebuffer].unbind();

            // swap frame buffers
            src_framebuffer = 1 - src_framebuffer;
            dst_framebuffer = 1 - dst_framebuffer;

            // next destination size
            dst_width /= 2;
            dst_height /= 2;
            src_texture_scale /= 2.;
        }

        unsafe {
            gl::Disable(gl::SCISSOR_TEST);
            gl::Scissor(0, 0, orig_width, orig_height);
            gl::Viewport(0, 0, orig_width, orig_height);
        }

        self.frame_buffers[0].color_texture.id
    }
}
