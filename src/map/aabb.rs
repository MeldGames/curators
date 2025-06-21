use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_min_size(min: Vec3, size: Vec3) -> Self {
        Self { min, max: min + size }
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmplt(self.max).all()
    }

    /// Returns the number of voxels inside the AABB
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    /// Does this AABB fit inside another?
    pub fn fits_inside(&self, container: &Aabb) -> bool {
        self.min.cmpge(container.min).all() && self.max.cmple(container.max).all()
    }

    pub fn rotate(self, rotation: Quat) -> Self {
        let center = self.center();
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let rotated_corners: Vec<Vec3> =
            corners.iter().map(|&c| rotation * (c - center) + center).collect();

        let mut min = rotated_corners[0];
        let mut max = rotated_corners[0];

        for &c in &rotated_corners[1..] {
            min = min.min(c);
            max = max.max(c);
        }

        Self { min, max }
    }
}
