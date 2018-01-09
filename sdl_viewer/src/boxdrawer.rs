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

const FRAGMENT_SHADER_FILLED_BOX: &'static str = include_str!("../shaders/filledBox.fs");
const VERTEX_SHADER_FILLED_BOX: &'static str = include_str!("../shaders/filledBox.vs");

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
    filled_program: GlProgram,
    filled_u_transform: GLint,
    filled_u_model_view_transform: GLint,
    filled_u_color: GLint,
    filled_vertex_array: GlVertexArray,
    _filled_buffer_position: GlBuffer,
    _filled_buffer_normals: GlBuffer,
    _filled_buffer_indices: GlBuffer,
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
        let box_vertices: [f32; 3*8] = [
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
                (box_vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&box_vertices[0]),
                gl::STATIC_DRAW,
            );
        }

        // define index buffer for 24 edges of the box
        let _outlines_buffer_indices = GlBuffer::new();
        _outlines_buffer_indices.bind(gl::ELEMENT_ARRAY_BUFFER);
        let outline_indices: [i32; 24] = [
            0,1, 1,2, 2,3, 3,0,		// front
		    4,5, 5,6, 6,7, 7,4,		// back
		    1,5, 6,2,				// right
		    4,0, 3,7,				// left
        ];
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (outline_indices.len() * mem::size_of::<i32>()) as GLsizeiptr,
                mem::transmute(&outline_indices[0]),
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

        let filled_program = GlProgram::new(VERTEX_SHADER_FILLED_BOX, FRAGMENT_SHADER_FILLED_BOX);  
        let filled_u_transform;
        let filled_u_model_view_transform;
        let filled_u_color;
    
        unsafe {
            gl::UseProgram(outlines_program.id);
            filled_u_transform = gl::GetUniformLocation(filled_program.id, c_str!("transform"));
            filled_u_model_view_transform = gl::GetUniformLocation(filled_program.id, c_str!("modelViewTransform"));
            filled_u_color = gl::GetUniformLocation(filled_program.id, c_str!("color"));
        }

        let filled_vertex_array = GlVertexArray::new();
        filled_vertex_array.bind();

        // vertex buffer for filled box
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

        unsafe{
            let pos_attr = gl::GetAttribLocation(filled_program.id, c_str!("aPos"));
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

        // vertex buffer for filled box: normals
        let _filled_buffer_normals = GlBuffer::new();
        _filled_buffer_normals.bind(gl::ARRAY_BUFFER);
        let per_face_vertex_normals: [f32; 3*4*6] = [
	        // front
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            0.0, 0.0, 1.0,
            // back
            0.0, 0.0, -1.0,
            0.0, 0.0, -1.0,
            0.0, 0.0, -1.0,
            0.0, 0.0, -1.0,
            // right
            1.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            // left
            -1.0, 0.0, 0.0,
            -1.0, 0.0, 0.0,
            -1.0, 0.0, 0.0,
            -1.0, 0.0, 0.0,
            // top
            0.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
            // bottom
            0.0, -1.0, 0.0,
            0.0, -1.0, 0.0,
            0.0, -1.0, 0.0,
            0.0, -1.0, 0.0,
        ];
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (per_face_vertex_normals.len() * mem::size_of::<f32>()) as GLsizeiptr,
                mem::transmute(&per_face_vertex_normals[0]),
                gl::STATIC_DRAW,
            );
        }

        unsafe {
            let normal_attr = gl::GetAttribLocation(filled_program.id, c_str!("aNormal"));
            gl::EnableVertexAttribArray(normal_attr as GLuint);
            gl::VertexAttribPointer(
                normal_attr as GLuint,
                3,
                gl::FLOAT,
                gl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        // define index buffer for 24 edges of the box
        let _filled_buffer_indices = GlBuffer::new();
        _filled_buffer_indices.bind(gl::ELEMENT_ARRAY_BUFFER);
        let mut filled_indices: [i32; 6*6] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        for i in 0..6 {
            let o = 6 * i;
            let v = 4 * i as i32;

            filled_indices[o + 0] = v + 0;
            filled_indices[o + 1] = v + 1;
            filled_indices[o + 2] = v + 2;

            filled_indices[o + 3] = v + 2;
            filled_indices[o + 4] = v + 3;
            filled_indices[o + 5] = v + 0;
        }
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (filled_indices.len() * mem::size_of::<i32>()) as GLsizeiptr,
                mem::transmute(&filled_indices[0]),
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
