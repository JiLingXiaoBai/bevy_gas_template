use bevy::math::Quat;
use bevy::prelude::World;
use bevy::shape::Sphere;
use bevy_gas::Random;
use bevy_gas_template::gameplay::randoms::SpatialRandom;

#[test]
fn bevy_math_and_shape_sampling_is_seeded_and_respects_geometry() {
    let mut first = SpatialRandom::from_seed(42);
    let mut second = SpatialRandom::from_seed(42);
    let sphere = Sphere::new(2.5);
    let tolerance = 1e-5;

    for _ in 0..16 {
        let rotation = first.from_rng::<Quat>();
        assert_eq!(rotation, second.from_rng::<Quat>());
        assert!(rotation.is_finite());
        assert!((rotation.length_squared() - 1.0).abs() < tolerance);

        let interior = first.sample_interior(&sphere);
        assert_eq!(interior, second.sample_interior(&sphere));
        assert!(interior.is_finite());
        assert!(interior.length() <= sphere.radius + tolerance);

        let boundary = first.sample_boundary(&sphere);
        assert_eq!(boundary, second.sample_boundary(&sphere));
        assert!(boundary.is_finite());
        assert!((boundary.length() - sphere.radius).abs() < tolerance);
    }
}

#[test]
fn resetting_spatial_seed_replays_mixed_samples() {
    let mut spatial = SpatialRandom::default();
    let sphere = Sphere::new(2.5);
    let first = (
        spatial.from_rng::<Quat>(),
        spatial.sample_interior(&sphere),
        spatial.sample_boundary(&sphere),
    );
    spatial.set_seed(SpatialRandom::DEFAULT_SEED);
    let replay = (
        spatial.from_rng::<Quat>(),
        spatial.sample_interior(&sphere),
        spatial.sample_boundary(&sphere),
    );
    assert_eq!(first, replay);
}

#[test]
fn spatial_and_gas_resources_do_not_advance_each_others_streams() {
    let mut world = World::new();
    world.insert_resource(Random::from_seed(42));
    world.insert_resource(SpatialRandom::from_seed(7));
    let mut expected_gas = Random::from_seed(42);
    let mut expected_spatial = SpatialRandom::from_seed(7);
    let sphere = Sphere::new(2.5);

    for tick in 0..32 {
        for _ in 0..tick % 5 {
            assert_eq!(
                world
                    .resource_mut::<SpatialRandom>()
                    .sample_interior(&sphere),
                expected_spatial.sample_interior(&sphere),
            );
        }
        for _ in 0..=tick % 3 {
            assert_eq!(
                world.resource_mut::<Random>().random_bool(0.37),
                expected_gas.random_bool(0.37),
            );
        }
        assert_eq!(
            world
                .resource_mut::<SpatialRandom>()
                .sample_boundary(&sphere),
            expected_spatial.sample_boundary(&sphere),
        );
    }
}
