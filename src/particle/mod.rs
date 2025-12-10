use rand::distr::Distribution;

use wgpu::Buffer;

use crate::{Context, mesh::Mesh, renderer::Instance};

pub struct ParticleEmitter {
    pub mesh: Mesh,
    instances: Vec<Instance>,
    instance_buffer: Buffer,
    pub config: EmitterConfig,
    pub position: Vec3,
    spawn_timer: f64
}
impl ParticleEmitter {
    pub fn update(&mut self, context: &mut Context) -> anyhow::Result<()> {
        if self.spawn_timer <= 0. {
            let instance = Instance {
                position: self.position.clone(),
                rotation: random_quat_in_cone(self.config.velocity_center_angle, self.config.velocity_angle_range.generate())
            };
        } else { self.spawn_timer -= context.delta; }

        Ok(())
    }
}

pub struct EmitterConfig {
    /// range of possible angular velocities
    pub angular_velocity: RandomRange,
    /// range of possible velocity angle variation from center
    pub velocity_angle_range: RandomRange,
    /// center angle around which particle velocities will vary
    pub velocity_center_angle: Quat,
    /// range of possible velocity magnitudes
    pub velocity_magnitudes: RandomRange,
    /// particles per second
    pub speed: RandomRange,
    /// size variation of particles,
    pub sizes: RandomRange,
    /// particle gravity
    pub gravity: RandomRange
}

pub enum RandomRange {
    /// mean, standard deviation
    NormalDistribution(f32, f32),
    /// min, max
    Uniform(f32, f32)
}
impl RandomRange {
    fn generate(&self) -> f32 {
        let mut rng = rand::rng();
        match self {
            RandomRange::NormalDistribution(mean, std_dev) => {
                let normal = rand_distr::Normal::new(*mean, *std_dev).expect("x_x :: error in normal distribution rng");
                normal.sample(&mut rng)
            },
            RandomRange::Uniform(min, max) => {
                rng.random::<f32>() * (max - min) + min
            }
        }
    }
}


use glam::{Quat, Vec3};
use rand::Rng;

/// Random quaternion in the orientation cone defined by `q_center` and `theta_max`.
fn random_quat_in_cone(q_center: Quat, theta_max: f32) -> Quat {
    let mut rng = rand::rng();

    // Pick random angle within the cone
    let u: f32 = rng.random(); // uniform 0..1
    let theta = (u).acos(); // invert cdf for uniform spherical cap
    let theta = theta * (theta_max / std::f32::consts::FRAC_PI_2).min(1.0); 

    // Random perpendicular axis
    let v = random_unit_vector();
    let axis = v.normalize();

    // Small rotation quaternion
    let q_offset = Quat::from_axis_angle(axis, theta);

    // Apply the offset to the central quaternion
    q_center * q_offset
}

/// Generate a uniform random direction on the sphere.
fn random_unit_vector() -> Vec3 {
    let mut rng = rand::rng();

    let z: f32 = rng.random_range(-1.0..1.0);
    let a: f32 = rng.random_range(0.0..std::f32::consts::TAU);

    let r = (1.0 - z * z).sqrt();

    Vec3::new(r * a.cos(), r * a.sin(), z)
}


pub struct Particle {
    instance: Instance,
    velocity 
}