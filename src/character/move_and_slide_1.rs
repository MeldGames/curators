use avian3d::prelude::*;
use bevy::prelude::*;

pub fn move_and_slide() {
    let mut remaining_distance = velocity.length() * delta_seconds;
    let radius = 0.5;
    let epsilon = 0.001;
    let collider = Collider::sphere(radius);
    let ignore_origin_penetration = true;

    // we loop 4 times because we may move then collide, then slide, then collide again
    for _ in 0..4 {
        if remaining_distance < epsilon || velocity.length_squared() < epsilon * epsilon {
            break;
        }

        let velocity_dir = velocity.normalize_or_zero();

        if let Some(first_hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            transform.rotation,
            Dir3::new(velocity_dir).unwrap_or(Dir3::Z),
            remaining_distance,
            ignore_origin_penetration,
            SpatialQueryFilter::default(),
        ) {
            // move to the point of impact
            let move_distance = (first_hit.time_of_impact - epsilon).max(0.0);
            transform.translation += velocity_dir * move_distance;

            // slide along the surface next move
            let normal = first_hit.normal1;
            velocity = velocity - normal * velocity.dot(normal);

            // prevents sticking
            transform.translation += normal * epsilon;

            // update remaining distance
            remaining_distance -= move_distance;
        } else {
            // no collision, move the full remaining distance
            transform.translation += velocity_dir * remaining_distance;
            break;
        }
    }
}
