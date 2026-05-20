//! Config validation: out-of-range fields are rejected; unknown starter names
//! are rejected; legacy POC config adapts cleanly.

use meme_garden_core::*;

fn good_toml() -> String {
    include_str!("../../../configs/presets/cooperation-vs-selfish-low.toml").to_string()
}

#[test]
fn good_config_validates() {
    let cfg = SimConfig::from_toml_str(&good_toml()).unwrap();
    cfg.validate().unwrap();
}

#[test]
fn out_of_range_probability_is_rejected() {
    let mut cfg = SimConfig::from_toml_str(&good_toml()).unwrap();
    cfg.transmission.base_rate = 1.5;
    let err = cfg.validate().unwrap_err();
    match err {
        ConfigError::OutOfRange { field, .. } => assert_eq!(field, "transmission.base_rate"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn traits_dist_must_sum_to_one() {
    let mut cfg = SimConfig::from_toml_str(&good_toml()).unwrap();
    cfg.agents.initial_traits_dist = [0.5, 0.5, 0.5, 0.5];
    let err = cfg.validate().unwrap_err();
    match err {
        ConfigError::OutOfRange { field, .. } => {
            assert_eq!(field, "agents.initial_traits_dist")
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unknown_starter_name_is_rejected() {
    let mut cfg = SimConfig::from_toml_str(&good_toml()).unwrap();
    cfg.memes.seed[0].name = "not_a_real_meme".into();
    let err = cfg.validate().unwrap_err();
    match err {
        ConfigError::UnknownStarterMeme(name) => assert_eq!(name, "not_a_real_meme"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_scarcity_level_is_rejected() {
    let mut cfg = SimConfig::from_toml_str(&good_toml()).unwrap();
    cfg.scarcity.level = "ridiculous".into();
    let err = cfg.validate().unwrap_err();
    match err {
        ConfigError::InvalidScarcityLevel(s) => assert_eq!(s, "ridiculous"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn legacy_config_adapts() {
    // The pre-MVP POC config shape.
    let legacy = r#"
        [world]
        width = 20
        height = 20

        [agents]
        count = 30
        starting_energy = 20.0
        metabolism = 0.5
        max_energy = 50.0

        [food]
        initial_density = 0.15
        regrowth_rate = 0.005
        energy_per_food = 8.0

        [meme]
        initial_carrier_fraction = 0.2
        transmissibility = 0.5
        share_threshold = 15.0
        share_amount = 4.0

        [run]
        seed = 1
    "#;
    let cfg = SimConfig::from_toml_str(legacy).expect("legacy config should adapt");
    assert_eq!(cfg.world.width, 20);
    assert_eq!(cfg.run.seed, 1);
    assert_eq!(cfg.memes.seed.len(), 1);
    assert_eq!(cfg.memes.seed[0].name, "share_with_allies");
    cfg.validate().expect("adapted config should validate");
}
