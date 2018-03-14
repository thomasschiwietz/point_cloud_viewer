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

use cgmath::{InnerSpace, Matrix, Matrix4, Point3, Vector3};
// use color::Color;
use graphic::{GlBuffer, GlProgram, GlVertexArray};
use opengl;
use opengl::types::{GLboolean, GLint, GLsizeiptr, GLuint};
use std::collections::HashMap;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use protobuf;
use proto;

use std::fs::File;
use std::io::{Cursor, Read};

const HM_FRAGMENT_SHADER: &str = include_str!("../shaders/heightmap.fs");
const HM_VERTEX_SHADER: &str = include_str!("../shaders/heightmap.vs");

pub struct GroundMap {
    pub proto: proto::GroundMap,
    pub tiles: HashMap<(i32, i32), usize>,
}

impl GroundMap {
    pub fn new(proto: proto::GroundMap) -> Self {
        let mut tiles = HashMap::new();
        for i in 0..proto.data.as_ref().unwrap().tile_data.len() {
            let tile = &proto.data.as_ref().unwrap().tile_data[i];
            tiles.insert((tile.tile_pos_x, tile.tile_pos_y), i);
        }
        GroundMap { proto, tiles }
    }

    pub fn get_resolution_m(&self) -> f32 {
        self.proto.resolution_m as f32
    }

    pub fn get_origin(&self) -> (f32, f32) {
        (self.proto.origin_x as f32, self.proto.origin_y as f32)
    }

    pub fn get_grid_size(&self) -> i32 {
        self.proto.data.as_ref().unwrap().width
    }

    pub fn get_tile_size(&self) -> i32 {
        self.proto.data.as_ref().unwrap().tile_size
    }

    pub fn get_default_value(&self) -> f32 {
        self.proto.data.as_ref().unwrap().default_value
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= self.get_grid_size() || y >= self.get_grid_size() {
            return self.get_default_value()
        }
        let tile_size = self.get_tile_size();
        let entry_opt = self.tiles.get(&(x / tile_size, y / tile_size));
        if entry_opt.is_some() {
            let tile = &self.proto.data.as_ref().unwrap().tile_data[*entry_opt.unwrap()];
            return tile.value[((x % tile_size) + (y % tile_size) * tile_size) as usize]
        }
        self.get_default_value()
    }

    pub fn get_world_pos(&self, x: i32, y: i32) -> Vector3<f32> {
        let (origin_x, origin_y) = self.get_origin();
        let resolution_m = self.get_resolution_m();
        Vector3::new(
            origin_x + (x as f32 * resolution_m),
            origin_y + (y as f32 * resolution_m),
            self.get_height(x, y)
        )
    }
}

pub struct HeightMapDrawer<'a> {
    pub origin: Point3<f32>,
    pub edge_length: f32,

    program: GlProgram<'a>,

    // Uniforms locations.
    u_transform: GLint,
    u_model_view_transform: GLint,
    u_color: GLint,

    // Vertex array and buffers
    vertex_array: GlVertexArray<'a>,
    buffer_position: GlBuffer<'a>,
    buffer_normal: GlBuffer<'a>,
    num_indices: usize,
}

impl<'a> HeightMapDrawer<'a> {
    pub fn new(gl: &'a opengl::Gl) -> Self {
        let program =
            GlProgram::new(gl, HM_VERTEX_SHADER, HM_FRAGMENT_SHADER);
        let u_transform;
        let u_model_view_transform;
        let u_color;

        unsafe {
            gl.UseProgram(program.id);
            u_transform = gl.GetUniformLocation(program.id, c_str!("transform"));
            u_model_view_transform = gl.GetUniformLocation(program.id, c_str!("modelViewTransform"));
            u_color = gl.GetUniformLocation(program.id, c_str!("color"));
        }

        let vertex_array = GlVertexArray::new(gl);
        vertex_array.bind();

        let buffer_position = GlBuffer::new_array_buffer(gl);

        let buffer_normal = GlBuffer::new_array_buffer(gl);

        unsafe {
            buffer_position.bind();
            let pos_attr = gl.GetAttribLocation(program.id, c_str!("aPos"));
            gl.EnableVertexAttribArray(pos_attr as GLuint);
            gl.VertexAttribPointer(
                pos_attr as GLuint,
                3,
                opengl::FLOAT,
                opengl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        unsafe {
            buffer_normal.bind();
            let normal_attr = gl.GetAttribLocation(program.id, c_str!("aNormal"));
            gl.EnableVertexAttribArray(normal_attr as GLuint);
            gl.VertexAttribPointer(
                normal_attr as GLuint,
                3,
                opengl::FLOAT,
                opengl::FALSE,
                3 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
        }

        let num_indices = 0;
        let origin = Point3::new(0., 0., 0.);
        let edge_length = 0.;

        HeightMapDrawer {
            origin,
            edge_length,
            program,
            u_transform,
            u_model_view_transform,
            u_color,
            vertex_array,
            buffer_position,
            buffer_normal,
            num_indices,
        }
    }

    fn linear_index(x: i32, y: i32, size: i32) -> usize {
        (x + y * size) as usize
    }

    fn compute_triangle_normal(v0: &Vector3<f32>, v1: &Vector3<f32>, v2: &Vector3<f32>) -> Vector3<f32> {
        let t0 = v2 - v0;
        let t1 = v1 - v0;
        let n = t0.cross(t1);
        -n.normalize()
    }

    pub fn load_proto(&mut self, file_name: String, use_vertex_normals: bool) {
        println!("loading height map {}", file_name);

        let ground_map_proto = {
            let mut data = Vec::new();
            File::open(&file_name).unwrap().read_to_end(&mut data).unwrap();
            protobuf::parse_from_reader::<proto::GroundMap>(&mut Cursor::new(data)).unwrap()
        };
        let ground_map = GroundMap::new(ground_map_proto);

        let grid_size = ground_map.get_grid_size();
        let tile_size = ground_map.get_tile_size();
        let (origin_x, origin_y) = ground_map.get_origin();
        let resolution_m = ground_map.get_resolution_m();
        let default_value = ground_map.get_default_value();

        self.edge_length = (grid_size - 1) as f32 * resolution_m;
        self.origin = Point3::new(origin_x, origin_y, 0.);

        // global list
        let mut triangle_vertices = Vec::new();
        let mut triangle_normals = Vec::new();

        for (&(tile_pos_x, tile_pos_y), _i) in &ground_map.tiles {
            let x_offset = tile_pos_x * tile_size;
            let y_offset = tile_pos_y * tile_size;
            let mut adapted_tile_size_x = tile_size;
            let mut adapted_tile_size_y = tile_size;
            let upper_limit_x = (tile_pos_x + 1) * tile_size;
            let upper_limit_y = (tile_pos_y + 1) * tile_size;
            if upper_limit_x >= grid_size {
                adapted_tile_size_x -= upper_limit_x - grid_size;
            }
            if upper_limit_y >= grid_size {
                adapted_tile_size_y -= upper_limit_y - grid_size;
            }

            let mut vertex_normals_vec = Vec::new();
            vertex_normals_vec.resize(((tile_size + 1) * (tile_size + 1)) as usize, Vec::new());

            let mut normals = Vec::new();
            for y in 0..adapted_tile_size_y {
                for x in 0..adapted_tile_size_x {
                    let i00 = HeightMapDrawer::linear_index(x, y, tile_size + 1);
                    let i10 = HeightMapDrawer::linear_index(x + 1, y, tile_size + 1);
                    let i01 = HeightMapDrawer::linear_index(x, y + 1, tile_size + 1);
                    let i11 = HeightMapDrawer::linear_index(x + 1, y + 1, tile_size + 1);

                    // get vertices
                    let v00 = ground_map.get_world_pos(x_offset + x, y_offset + y);
                    let v10 = ground_map.get_world_pos(x_offset + x + 1, y_offset + y);
                    let v01 = ground_map.get_world_pos(x_offset + x, y_offset + y + 1);
                    let v11 = ground_map.get_world_pos(x_offset + x + 1, y_offset + y + 1);

                    // skip triangles with undefined height values
                    // if v00.z.is_nan() || v10.z.is_nan() || v01.z.is_nan() || v11.z.is_nan() { 
                    //     continue;
                    // }

                    if v00.z == default_value || v01.z == default_value || v10.z == default_value || v11.z == default_value {
                        continue;
                    }

                    // lower triangle
                    triangle_vertices.push(v00);
                    triangle_vertices.push(v10);
                    triangle_vertices.push(v11);
                    let normal0 = HeightMapDrawer::compute_triangle_normal(&v00, &v10, &v11);
                    normals.push(normal0);
                    normals.push(normal0);
                    normals.push(normal0);
                    vertex_normals_vec[i00].push(normal0);
                    vertex_normals_vec[i10].push(normal0);
                    vertex_normals_vec[i11].push(normal0);

                    // upper triangle
                    triangle_vertices.push(v00);
                    triangle_vertices.push(v11);
                    triangle_vertices.push(v01);
                    let normal1 = HeightMapDrawer::compute_triangle_normal(&v00, &v11, &v01);
                    normals.push(normal1);
                    normals.push(normal1);
                    normals.push(normal1);
                    vertex_normals_vec[i00].push(normal1);
                    vertex_normals_vec[i11].push(normal1);
                    vertex_normals_vec[i01].push(normal1);
                }
            }

            // normalize vertex normals
            if use_vertex_normals {
                let mut vertex_normals = Vec::new();
                for vn in &vertex_normals_vec {
                    let mut vertex_normal = Vector3::new(0., 0., 0.);
                    for normal in vn {
                        vertex_normal += *normal;
                    }
                    if vn.len() > 0 {
                        vertex_normal /= vn.len() as f32;
                    }
                    vertex_normals.push(vertex_normal);
                }

                // recreate triangle normals from vertex normals
                normals.clear();
                for y in 0..adapted_tile_size_y {
                    for x in 0..adapted_tile_size_x {
                        // get indices
                        let i00 = HeightMapDrawer::linear_index(x, y, tile_size + 1);
                        let i10 = HeightMapDrawer::linear_index(x + 1, y, tile_size + 1);
                        let i01 = HeightMapDrawer::linear_index(x, y + 1, tile_size + 1);
                        let i11 = HeightMapDrawer::linear_index(x + 1, y + 1, tile_size + 1);

                        // get vertices
                        let v00 = ground_map.get_world_pos(x_offset + x, y_offset + y);
                        let v10 = ground_map.get_world_pos(x_offset + x + 1, y_offset + y);
                        let v01 = ground_map.get_world_pos(x_offset + x, y_offset + y + 1);
                        let v11 = ground_map.get_world_pos(x_offset + x + 1, y_offset + y + 1);

                        // skip triangles with undefined height values
                        // if v00.z.is_nan() || v10.z.is_nan() || v01.z.is_nan() || v11.z.is_nan() { 
                        //     continue;
                        // }

                        if v00.z == default_value || v01.z == default_value || v10.z == default_value || v11.z == default_value {
                            continue;
                        }

                        // lower triangle
                        normals.push(vertex_normals[i00]);
                        normals.push(vertex_normals[i10]);
                        normals.push(vertex_normals[i11]);

                        // upper triangle
                        normals.push(vertex_normals[i00]);
                        normals.push(vertex_normals[i11]);
                        normals.push(vertex_normals[i01]);
                    }
                }
            }

            triangle_normals.extend(normals);

            // TODO: create index buffer and unique vertex and normal buffer (not possible triangle normals!)
        }

        self.num_indices = triangle_vertices.len();
        //println!("{:?}", self.triangle_vertices);

        println!("number of triangles {}", self.num_indices / 3);

        self.vertex_array.bind();

        self.buffer_position.bind();
        unsafe {
            self.program.gl.BufferData(
                opengl::ARRAY_BUFFER,
                (triangle_vertices.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                triangle_vertices.as_ptr() as *const c_void,
                opengl::STATIC_DRAW,
            );
        }

        self.buffer_normal.bind();
        unsafe {
            self.program.gl.BufferData(
                opengl::ARRAY_BUFFER,
                (triangle_normals.len() * 3 * mem::size_of::<f32>()) as GLsizeiptr,
                triangle_normals.as_ptr() as *const c_void,
                opengl::STATIC_DRAW,
            );
        }

    }

    pub fn draw(&self, color: &Vec<f32>, world_to_view: &Matrix4<f32>, world_to_gl: &Matrix4<f32>) {
        self.vertex_array.bind();

        unsafe {
            self.program.gl.UseProgram(self.program.id);
            self.program.gl.UniformMatrix4fv(self.u_transform, 1, false as GLboolean, world_to_gl.as_ptr());
            self.program.gl.UniformMatrix4fv(self.u_model_view_transform, 1, false as GLboolean, world_to_view.as_ptr());
            self.program.gl.Uniform4fv(self.u_color, 1, color.as_ptr());
            self.program.gl.DrawArrays(opengl::TRIANGLES, 0, self.num_indices as i32);
        }
    }
}
