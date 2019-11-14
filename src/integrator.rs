use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use rand::rngs::SmallRng;
use rand::Rng;

use crate::film::ChannelSample;
use crate::hitable::WShadingPoint;
use crate::material::MaterialHandle;
use crate::math::{f32x4, Vec2u, Vec3};
use crate::ray::Ray;
use crate::spectrum::Srgb;
use crate::world::World;

pub trait Integrator: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn integrate(
        &self,
        world: &World,
        rng: &mut SmallRng,
        depth: usize,
        material: MaterialHandle,
        intersection: WShadingPoint,
        bump: &Bump,
        spawned_rays: &mut BumpVec<Ray>,
        output_samples: &mut BumpVec<(Vec2u, ChannelSample)>,
    );
}

#[derive(Clone, Copy)]
pub struct PathTracingIntegrator {
    pub max_bounces: usize,
}

impl Integrator for PathTracingIntegrator {
    fn integrate(
        &self,
        world: &World,
        rng: &mut SmallRng,
        depth: usize,
        material: MaterialHandle,
        mut intersection: WShadingPoint,
        bump: &Bump,
        spawned_rays: &mut BumpVec<Ray>,
        output_samples: &mut BumpVec<(Vec2u, ChannelSample)>,
    ) {
        let wi = intersection.ray.dir;
        let material = world.materials.get(material);

        let bsdf = material.get_bsdf_at(&intersection, bump);

        intersection.ray.radiance += bsdf.le(-wi, &intersection) * intersection.ray.throughput;

        let scattering_event = bsdf.scatter(wi, &intersection, rng);

        if let Some(se) = scattering_event {
            let ndl = se.wi.dot(intersection.normal).abs();

            let mut new_throughput = intersection.ray.throughput * se.f / se.pdf * ndl;

            let roulette_factor = if depth > 2 {
                let roulette_factor =
                    (f32x4::ONE - intersection.ray.throughput.max_channel()).max(f32x4::from(0.05));

                new_throughput /= f32x4::ONE - roulette_factor;

                roulette_factor
            } else {
                f32x4::ZERO
            };

            let mut new_rays: [Ray; 4] = intersection.create_rays(se.wi).into();
            let throughputs: [Srgb; 4] = new_throughput.into();

            if depth == 0 {
                let normals: [Vec3; 4] = intersection.normal.into();
                for (ray, normal) in new_rays.iter().zip(normals.iter()) {
                    if ray.valid {
                        output_samples.push((ray.tile_coord, ChannelSample::Alpha(1.0)));
                        output_samples.push((ray.tile_coord, ChannelSample::WorldNormal(*normal)));
                    }
                }
            }

            for ((ray, new_throughput), roulette_factor) in new_rays
                .iter_mut()
                .zip(throughputs.iter())
                .zip(roulette_factor.as_ref().iter())
            {
                if ray.valid {
                    if depth >= self.max_bounces || rng.gen::<f32>() < *roulette_factor {
                        output_samples.push((ray.tile_coord, ChannelSample::Color(ray.radiance)));
                    } else {
                        if !new_throughput.is_nan() {
                            ray.throughput = *new_throughput;
                        }

                        spawned_rays.push(*ray);
                    }
                }
            }
        } else {
            let final_rays: [Ray; 4] = intersection.ray.into();

            for ray in final_rays.iter() {
                if ray.valid {
                    let sample = if depth == 0 {
                        ChannelSample::Background(ray.radiance)
                    } else {
                        ChannelSample::Color(ray.radiance)
                    };

                    output_samples.push((ray.tile_coord, sample));
                }
            }
        }
    }
}
