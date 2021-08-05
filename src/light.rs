use arcana::na;

#[derive(Clone, Copy, Debug)]
pub struct DirLight {
    pub dir: na::Vector3<f32>,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    pub color: [f32; 3],
}
