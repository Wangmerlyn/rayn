use crate::camera::CameraStore;
use crate::hitable::HitableStore;
use crate::light::Light;
use crate::material::MaterialStore;
use crate::volume::VolumeParams;
use crate::sky::Sky;

pub struct World {
    pub hitables: HitableStore,
    pub lights: Vec<Box<dyn Light>>,
    pub materials: MaterialStore,
    pub cameras: CameraStore,
    pub volume_params: VolumeParams,
    pub sky: Sky,
}
