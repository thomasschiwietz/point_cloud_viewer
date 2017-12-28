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

pub struct GlFramebuffer {
    frame_buffer_id: GLuint,
    color_texture_id: GLuint,
    depth_buffer_id: GLuint,
}

impl GlFramebuffer {
    pub fn new() -> Self {
        let mut frame_buffer_id = 0;
        let mut color_texture_id = 0;
        let mut depth_buffer_id = 0;

        let width = 800;
        let height = 600;

        unsafe {
            gl::GenFramebuffers(1, &mut frame_buffer_id);
            println!("fb {}, err {}", frame_buffer_id, gl::GetError());

            gl::BindFramebuffer(gl::FRAMEBUFFER, frame_buffer_id);

            // create color buffer
            gl::GenTextures(1, &mut color_texture_id);

            // "Bind" the newly created texture : all future texture functions will modify this texture
            gl::BindTexture(gl::TEXTURE_2D, color_texture_id);

            // Give an empty image to OpenGL ( the last "0" )
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, width, height, 0, gl::RGB, gl::UNSIGNED_BYTE, ptr::null());

            // Poor filtering. Needed !
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);

            println!("color {}, err {}", color_texture_id, gl::GetError());

            // The depth buffer
            gl::GenRenderbuffers(1, &mut depth_buffer_id);
            gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer_id);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, depth_buffer_id);

            println!("depth {}, err {}", depth_buffer_id, gl::GetError());

            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, color_texture_id, 0);

            // Set the list of draw buffers.
            let mut draw_buffers: [GLenum; 1] = [gl::COLOR_ATTACHMENT0];
            gl::DrawBuffers(draw_buffers.len() as i32, &draw_buffers[0]); // "1" is the size of DrawBuffers

            println!("err {}", gl::GetError());

            if (gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE) {
                println!("fb not complete");                
            }

            // unbind
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            
        }
        GlFramebuffer { frame_buffer_id, color_texture_id, depth_buffer_id }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.frame_buffer_id);
            gl::Viewport(0, 0, 800, 600);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

impl Drop for GlFramebuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.frame_buffer_id);
            gl::DeleteTextures(1, &self.color_texture_id);
            gl::DeleteRenderbuffers(1, &self.depth_buffer_id);
        }
    }
}
