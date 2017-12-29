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
use std::cmp;
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
    u_max_size_step: GLint,
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
        let u_max_size_step;
        unsafe {
            gl::UseProgram(program_max.id);
            u_max_texture_id = gl::GetUniformLocation(program_max.id, c_str!("aTex"));
            u_max_size_step = gl::GetUniformLocation(program_max.id, c_str!("size_step"));
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
            u_max_size_step,
        }        
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        self.frame_buffers[0].set_size(width, height);
        self.frame_buffers[1].set_size(width, height);
    }

    // return texture_id of result
    pub fn reduce_max(&self, depth_texture_id: GLuint, max_steps: i32) -> (GLuint, i32, i32) {
        // texture dimensions of texture_ID and internal frame buffer must match!
        // save current viewport

        let orig_width = self.frame_buffers[0].width;
        let orig_height = self.frame_buffers[0].height;

        // step in normalized coordinates to access neighboring texel
        let tex_step_x = 1. / orig_width as f32;
        let tex_step_y = 1. / orig_height as f32;

        let mut dst_width = orig_width / 2;
        let mut dst_height = orig_height / 2;
        let mut src_texture_scale = 1.;

        let mut src_framebuffer = 1;
        let mut dst_framebuffer = 0;

        unsafe {
            gl::Enable(gl::SCISSOR_TEST);
            gl::UseProgram(self.program_max.id);

            // bind texture to texture unit 0
            gl::Uniform1i(self.u_max_texture_id, 0);
            gl::ActiveTexture(gl::TEXTURE0 + 0);
        }

        let steps = cmp::max(max_steps, 1);

        for i in 0..steps {       // arbitrary limit
            // setup target frame buffer
            self.frame_buffers[dst_framebuffer].bind();
            unsafe {
                gl::Viewport(0, 0, dst_width, dst_height);
                gl::Scissor(0, 0, dst_width, dst_height);

                println!("{}: size {}, {}", i, dst_width, dst_height);

                // clear is not necessary

                // set step and scaling uniform
                gl::Uniform4f(self.u_max_size_step, dst_width as f32 * 2., dst_height as f32 * 2., tex_step_x, tex_step_y);

                // first time use provided depth texture, otherwise source frame buffer color texture
                let texture_id;
                if i == 0 {
                    texture_id = depth_texture_id;
                }
                else {
                    texture_id = self.frame_buffers[src_framebuffer].color_texture.id;                    
                }
                gl::BindTexture(gl::TEXTURE_2D, texture_id);                    

                self.quad_buffer.draw();
            }

            self.frame_buffers[dst_framebuffer].unbind();

            // swap frame buffers
            src_framebuffer = 1 - src_framebuffer;
            dst_framebuffer = 1 - dst_framebuffer;

            // next destination size
            dst_width /= 2;
            dst_height /= 2;
            src_texture_scale /= 2.;

            // limit reduction
            if dst_width < 8 || dst_height < 8 {
                break;
            }
        }

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::Disable(gl::SCISSOR_TEST);
            gl::Scissor(0, 0, orig_width, orig_height);
            gl::Viewport(0, 0, orig_width, orig_height);
        }

        (self.frame_buffers[src_framebuffer].color_texture.id, dst_width * 2, dst_height * 2)
    }
}
