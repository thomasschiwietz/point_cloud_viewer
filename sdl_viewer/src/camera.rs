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

use cgmath::{Angle, Decomposed, Deg, InnerSpace, Matrix4, One, Quaternion, Rad, Rotation,
             Rotation3, Transform, Vector3, Zero, PerspectiveFov, Ortho};

use gl;
use std::f32;

#[derive(Debug)]
pub struct Camera {
    pub moving_backward: bool,
    pub moving_forward: bool,
    pub moving_left: bool,
    pub moving_right: bool,
    pub moving_down: bool,
    pub moving_up: bool,
    pub width: i32,
    pub height: i32,

    movement_speed: f32,
    theta: Rad<f32>,
    phi: Rad<f32>,

    moved: bool,
    transform: Decomposed<Vector3<f32>, Quaternion<f32>>,

    projection_matrix: Matrix4<f32>,
    ortho_projection: bool,      // perspective = false, ortho = false
}

impl Camera {
    pub fn new(width: i32, height: i32, ortho: bool) -> Self {
        let mut camera = Camera {
            movement_speed: 0.3, //1.5,
            moving_backward: false,
            moving_forward: false,
            moving_left: false,
            moving_right: false,
            moving_down: false,
            moving_up: false,
            moved: true,
            theta: Rad(0.),
            phi: Rad(0.),
            transform: Decomposed {
                scale: 1.,
                rot: Quaternion::one(),
                disp: Vector3::new(0., 0., 300.),
            },

            // These will be set by set_size().
            projection_matrix: One::one(),
            width: 0,
            height: 0,
            ortho_projection: ortho,
        };
        camera.set_size(width, height);
        camera
    }

    pub fn set_pos_rot(&mut self, pos: &Vector3<f32>, theta: Deg<f32>, phi: Deg<f32>) {
        self.theta = Rad::from(theta);
        self.phi = Rad::from(phi);
        self.transform.disp = *pos;
    }

    pub fn get_pos(&self) -> Vector3<f32> {
        self.transform.disp
    }

    pub fn get_theta(&self) -> Rad<f32> {
        self.theta
    }

    pub fn get_phi(&self) -> Rad<f32> {
        self.phi
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        let aspect = width as f32 / height as f32;
        if !self.ortho_projection {
            self.projection_matrix = Matrix4::from(PerspectiveFov{fovy: Rad::from(Deg(45.)), aspect: aspect, near: 0.1, far: 10000.});
        } else {
            let ext = 80.0;
            self.projection_matrix = Matrix4::from(Ortho{left: -ext, right: ext, top: 2. * ext/aspect, bottom: 0., near: -10000., far: 10000.0});
        }
    }

    pub fn get_world_to_camera(&self) -> Matrix4<f32> {
        let world_to_camera: Matrix4<f32> = self.transform.inverse_transform().unwrap().into();
        world_to_camera
    }

    pub fn get_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix
    }

    pub fn get_world_to_gl(&self) -> Matrix4<f32> {
        let world_to_camera: Matrix4<f32> = self.transform.inverse_transform().unwrap().into();
        self.projection_matrix * world_to_camera
    }

    /// Update the camera position for the current frame. Returns true if the camera moved in this
    /// step.
    pub fn update(&mut self) -> bool {
        let mut moved = self.moved;
        self.moved = false;

        let mut pan = Vector3::zero();
        if self.moving_right {
            pan.x += 1.;
        }
        if self.moving_left {
            pan.x -= 1.;
        }
        if self.moving_backward {
            pan.z += 1.;
        }
        if self.moving_forward {
            pan.z -= 1.;
        }
        if self.moving_up {
            pan.y += 1.;
        }
        if self.moving_down {
            pan.y -= 1.;
        }

        if pan.magnitude2() > 0. {
            moved = true;
            let translation = self.transform
                .rot
                .rotate_vector(pan.normalize() * self.movement_speed);
            self.transform.disp += translation;
        }

        let rotation_z = Quaternion::from_angle_z(self.theta);
        let rotation_x = Quaternion::from_angle_x(self.phi);
        self.transform.rot = rotation_z * rotation_x;
        moved
    }

    pub fn mouse_drag(&mut self, delta_x: i32, delta_y: i32) {
        self.moved = true;
        self.theta -= Rad(2. * f32::consts::PI * delta_x as f32 / self.width as f32);
        self.phi -= Rad(2. * f32::consts::PI * delta_y as f32 / self.height as f32);
    }

    pub fn mouse_wheel(&mut self, delta: i32) {
        let sign = delta.signum() as f32;
        self.movement_speed += sign * 0.1 * self.movement_speed;
        self.movement_speed = self.movement_speed.max(0.01);
    }
}
