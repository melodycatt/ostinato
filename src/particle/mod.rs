use std::fmt::Debug;

use rand::distr::Distribution;

use crate::{
    Context,
    renderer::{Instance, Renderable, Renderer},
    resources::{InstancedMesh, VertexBuffer},
};

pub struct ParticleEmitter<V: VertexBuffer + Debug> {
    pub mesh: InstancedMesh<V>,
    particles: Vec<Particle>,
    pub config: EmitterConfig,
    pub position: Vec3,
    spawn_timer: f32,
}
impl<V: VertexBuffer + Debug> ParticleEmitter<V> {
    pub fn new(mesh: InstancedMesh<V>, config: EmitterConfig, position: Vec3) -> Self {
        Self {
            mesh,
            particles: Vec::new(),
            config,
            position,
            spawn_timer: 0.,
        }
    }
    pub fn update(&mut self, context: &mut Context) -> anyhow::Result<()> {
        if self.spawn_timer <= 0. {
            let instance = Instance {
                pivot: Vec3::ZERO,
                position: self.position,
                scale: self.config.scales.generate() * Vec3::splat(1.0),
                rotation: random_quat_in_cone(
                    self.config.velocity_center_angle,
                    self.config.velocity_angle_range.generate(),
                ),
            };

            self.particles.push(Particle {
                instance,
                velocity_dir: random_quat_in_cone(
                    self.config.velocity_center_angle,
                    self.config.velocity_angle_range.generate(),
                ),
                velocity_magnitude: self.config.velocity_magnitudes.generate(),
                angular_velocity: self.config.angular_velocities.generate(),
                gravity: self.config.gravity_direction * self.config.gravity_magnitudes.generate(),
            });
            self.spawn_timer = self.config.speed.generate();
        } else {
            self.spawn_timer -= context.delta as f32;
        }

        for i in self.particles.iter_mut() {
            i.update(self.config.air_resistance, context.delta as f32);
        }

        self.mesh.instances.instances = self
            .particles
            .iter()
            .map(|p| p.instance)
            .collect::<Vec<_>>();
        self.mesh
            .instances
            .update_buffer(&context.renderer.device, &context.renderer.queue);
        Ok(())
    }
    pub fn render(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        renderer: &mut Renderer,
    ) -> anyhow::Result<()> {
        self.mesh.draw(pass, &[], renderer)
    }
}

pub struct EmitterConfig {
    /// range of possible angular velocities
    pub angular_velocities: RandomRange,
    /// range of possible velocity angle variation from center
    pub velocity_angle_range: RandomRange,
    /// center angle around which particle velocities will vary
    pub velocity_center_angle: Quat,
    /// range of possible velocity magnitudes
    pub velocity_magnitudes: RandomRange,
    /// range of possible rotations
    pub rotations: RandomRange,
    /// particles per second
    pub speed: RandomRange,
    /// size variation of particles,
    pub scales: RandomRange,
    /// particle gravity
    pub gravity_magnitudes: RandomRange,
    pub gravity_direction: Vec3,
    pub air_resistance: f32,
}

pub enum RandomRange {
    /// mean, standard deviation
    NormalDistribution(f32, f32),
    /// min, max
    Uniform(f32, f32),
    Constant(f32),
}
impl RandomRange {
    fn generate(&self) -> f32 {
        let mut rng = rand::rng();
        match *self {
            RandomRange::NormalDistribution(mean, std_dev) => {
                let normal = rand_distr::Normal::new(mean, std_dev)
                    .expect("x_x :: error in normal distribution rng");
                normal.sample(&mut rng)
            }
            RandomRange::Uniform(min, max) => rng.random::<f32>() * (max - min) + min,
            RandomRange::Constant(val) => val,
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
    velocity_dir: glam::Quat,
    velocity_magnitude: f32,
    angular_velocity: f32,
    gravity: Vec3,
}
impl Particle {
    fn update(&mut self, air_resistance: f32, dt: f32) {
        let mut vel = self.velocity_dir * Vec3::Z * self.velocity_magnitude;

        vel += self.gravity * dt;

        if air_resistance > 0.0 {
            vel -= vel * air_resistance * dt;
        }

        let speed = vel.length();

        if speed > 1e-6 {
            self.velocity_dir = Quat::from_rotation_arc(Vec3::Z, vel.normalize());
            self.velocity_magnitude = speed;
        } else {
            self.velocity_magnitude = 0.0;
        }

        self.instance.position += self.velocity_dir * Vec3::Z * (self.velocity_magnitude * dt);
        let (axis, angle) = self.instance.rotation.to_axis_angle();
        self.instance.rotation = Quat::from_axis_angle(axis, angle + self.angular_velocity * dt);
        dbg!(self.velocity_magnitude, "AAAA!!!!");
    }
}
