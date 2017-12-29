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

//! Higher level abstractions around core OpenGL concepts.

use gl;
use gl::types::{GLenum, GLuint};
use glhelper::{compile_shader, link_program};
use std::str;
use std::ptr;

pub struct GlProgram {
    pub id: GLuint,
}

impl GlProgram {
    pub fn new(vertex_shader: &str, fragment_shader: &str) -> Self {
        let vertex_shader_id = compile_shader(vertex_shader, gl::VERTEX_SHADER);
        let fragment_shader_id = compile_shader(fragment_shader, gl::FRAGMENT_SHADER);
        let id = link_program(vertex_shader_id, fragment_shader_id);

        // TODO(hrapp): Pull out some saner abstractions around program compilation.
        unsafe {
            gl::DeleteShader(vertex_shader_id);
            gl::DeleteShader(fragment_shader_id);
        }

        GlProgram { id }
    }
}

impl Drop for GlProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub struct GlBuffer {
    id: GLuint,
}

impl GlBuffer {
    pub fn new() -> Self {
        let mut id = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        GlBuffer { id }
    }

    pub fn bind(&self, buffer_type: GLuint) {
        unsafe {
            gl::BindBuffer(buffer_type, self.id);
        }
    }
}

impl Drop for GlBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

pub struct GlVertexArray {
    id: GLuint,
}

impl GlVertexArray {
    pub fn new() -> Self {
        let mut id = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        GlVertexArray { id }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }
}

impl Drop for GlVertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}

pub struct GlQuery {
    id: GLuint,
    target: GLuint,
}

impl GlQuery {
    pub fn new() -> Self {
        let mut id = 0;
        let error;
        unsafe {
            gl::GenQueries(1, &mut id);
            error = gl::GetError();
        }
        //println!("create query {}, error = {}", id, error);
        let target = 0;
        GlQuery { id, target }
    }

    fn begin(&self) {
        let error;
        unsafe {
            gl::BeginQuery(self.target, self.id);
            error = gl::GetError();
        }
        //println!("begin query {}, error = {}", self.target, error);
    }

    pub fn begin_samples_passed(&mut self) {
        self.target = gl::SAMPLES_PASSED;
        self.begin();
    }

    pub fn end(&self) {
        let error;
        unsafe {
            gl::EndQuery(self.target);
            error = gl::GetError();
        }
        //println!("end query {}, error = {}", self.target, error);
    }

    pub fn query_samples_passed(&mut self) -> u32 {
        let mut result: u32 = 0;
        let error;
        unsafe {
            gl::GetQueryObjectuiv(self.id, gl::QUERY_RESULT, &mut result);
            error = gl::GetError();
        }
        // println!("result {}, error = {}", result, error);
        result
    }
}

impl Drop for GlQuery {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteQueries(1, &self.id);
        }
        //println!("delete query {}", self.id);        
    }
}

pub enum TextureType {
    Uninitialized,
    ColorRGB8,
    ColorR32F,
    Depth,
}

pub struct GlTexture {
    pub id: GLuint,
    texture_type: TextureType,
}

impl GlTexture {
    pub fn new(width: i32, height: i32, texture_type: TextureType) -> Self {
        let id = GlTexture::create_texture(width, height, &texture_type);

        GlTexture { id, texture_type }
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        // check size and return if unchanged
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }

        self.id = GlTexture::create_texture(width, height, &self.texture_type);
    }

    fn create_texture(width: i32, height: i32, texture_type: &TextureType) -> GLuint {
        let mut id = 0;
        unsafe {
            match *texture_type {
                TextureType::ColorRGB8 => {
                    gl::GenTextures(1, &mut id);
                    gl::BindTexture(gl::TEXTURE_2D, id);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, width, height, 0, gl::RGB, gl::UNSIGNED_BYTE, ptr::null());
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                },
                TextureType::ColorR32F => {
                    gl::GenTextures(1, &mut id);
                    gl::BindTexture(gl::TEXTURE_2D, id);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::R32F as i32, width, height, 0, gl::RED, gl::FLOAT, ptr::null());
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                },
                TextureType::Depth => {
                    gl::GenTextures(1, &mut id);
                    gl::BindTexture(gl::TEXTURE_2D, id);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT32 as i32, width, height, 0, gl::DEPTH_COMPONENT, gl::FLOAT, ptr::null());
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_FUNC, gl::LEQUAL as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_MODE, gl::NONE as i32);
                },
                TextureType::Uninitialized => {},
            };

            gl::BindTexture(gl::TEXTURE_2D, 0);            
        }
        id
    }
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}

pub struct GlFramebuffer {
    frame_buffer_id: GLuint,
    pub color_texture: GlTexture,
    pub depth_texture: GlTexture,
    pub width: i32,
    pub height: i32,
}

impl GlFramebuffer {
    pub fn new(width: i32, height: i32, color_type: TextureType, depth_type: TextureType) -> Self {
        let mut frame_buffer_id = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut frame_buffer_id);
        }

        let color_texture = GlTexture::new(width, height, color_type);
        let depth_texture = GlTexture::new(width, height, depth_type);

        GlFramebuffer::attach(frame_buffer_id, color_texture.id, depth_texture.id);

        GlFramebuffer { frame_buffer_id, color_texture, depth_texture, width, height }
    }

    fn attach(frame_buffer_id: GLuint, color_texture_id: GLuint, depth_texture_id: GLuint) {
         unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, frame_buffer_id);

            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, color_texture_id, 0);
            if depth_texture_id != 0 {
                gl::FramebufferTexture(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, depth_texture_id, 0);
            }

            // error checks
            if (gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE) {
                println!("fb not complete");                
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }       
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        self.unbind();
        self.color_texture.set_size(width, height);
        if self.depth_texture.id != 0 {
            self.depth_texture.set_size(width, height);
        }
        GlFramebuffer::attach(self.frame_buffer_id, self.color_texture.id, self.depth_texture.id);
        self.width = width;
        self.height = height;
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.frame_buffer_id);
            gl::Viewport(0, 0, self.width, self.height);

            //let mut draw_buffers: [GLenum; 1] = [gl::COLOR_ATTACHMENT0];
            //gl::DrawBuffers(draw_buffers.len() as i32, &draw_buffers[0]);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            // reset original viewport!
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            //gl::DrawBuffer(gl::BACK);
        }
    }
}

impl Drop for GlFramebuffer {
    fn drop(&mut self) {
        unsafe {
            self.unbind();
            gl::DeleteFramebuffers(1, &self.frame_buffer_id);
        }
    }
}
