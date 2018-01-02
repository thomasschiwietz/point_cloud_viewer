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

        let width2 = (width as usize).next_power_of_two() / 2;
        let height2 = (height as usize).next_power_of_two() / 2;

        let frame_buffers = [
            GlFramebuffer::new(width2 as i32, height2 as i32, TextureType::ColorR32F, TextureType::Uninitialized), 
            GlFramebuffer::new(width2 as i32, height2 as i32, TextureType::ColorR32F, TextureType::Uninitialized),
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
        let width2 = (width as usize).next_power_of_two() / 2;
        let height2 = (height as usize).next_power_of_two() / 2;

        self.frame_buffers[0].set_size(width2 as i32, height2 as i32);
        self.frame_buffers[1].set_size(width2 as i32, height2 as i32);
    }

    // return texture_id of result
    pub fn reduce_max(&self, depth_texture_id: GLuint, tex_width: i32, tex_height: i32, max_steps: i32) -> (GLuint, f32) {
        // TDO(tschiwietz): save current viewport

        // frame buffer size
        let fb_width = self.frame_buffers[0].width;
        let fb_height = self.frame_buffers[0].height;

        // step in normalized coordinates to access neighboring texel
        let fb_step_x = 1. / fb_width as f32;
        let fb_step_y = 1. / fb_height as f32;

        // fb_width and height are already the next smaller power of two
        let mut dst_width = fb_width;
        let mut dst_height = fb_height;
        let mut src_texture_scale = 1.;

        // index into double buffer for source and destination
        let mut src_framebuffer = 1;
        let mut dst_framebuffer = 0;

        unsafe {
            gl::UseProgram(self.program_max.id);

            // bind texture to texture unit 0
            gl::Uniform1i(self.u_max_texture_id, 0);
            gl::ActiveTexture(gl::TEXTURE0 + 0);
        }

        let steps = cmp::max(max_steps, 1);

        // set source to input texture dimensions
        let mut src_width = tex_width as f32;
        let mut src_height = tex_height as f32;
        let mut src_step_x = 1. / src_width;
        let mut src_step_y = 1. / src_height;

        for i in 0..steps {       // arbitrary limit
            // setup target frame buffer
            self.frame_buffers[dst_framebuffer].bind();
            unsafe {
                gl::Viewport(0, 0, dst_width, dst_height);

                // set step and scaling uniform
                gl::Uniform4f(self.u_max_size_step, src_width, src_height, src_step_x, src_step_y);

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

            // update source dimensions from framebuffer
            src_width = dst_width as f32;
            src_height = dst_height as f32;
            src_step_x = fb_step_x;
            src_step_y = fb_step_y;

            // next destination size. 
            dst_width /= 2;
            dst_height /= 2;
            src_texture_scale /= 2.;

            // limit reduction
            if dst_width < 8 || dst_height < 8 {
                break;
            }
        }

        // undo last destination update
        dst_width *= 2;
        dst_height *= 2;
        src_texture_scale *= 2.;

        // download data

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::Viewport(0, 0, tex_width, tex_height);
        }

        (self.frame_buffers[src_framebuffer].color_texture.id, src_texture_scale)
    }
}
