use plan_atlas::config::{Config, Paths};
use plan_atlas::taxonomy::Taxonomy;

#[test]
fn resolve_respects_env() {
    let tmp = tempfile::tempdir().unwrap();
    let old = std::env::var_os("PLAN_ATLAS_HOME");
    // SAFETY: this is the only test in this binary that touches the environment,
    // and it restores the previous value before returning.
    unsafe { std::env::set_var("PLAN_ATLAS_HOME", tmp.path()) };
    let paths = Paths::resolve();
    unsafe {
        match old {
            Some(v) => std::env::set_var("PLAN_ATLAS_HOME", v),
            None => std::env::remove_var("PLAN_ATLAS_HOME"),
        }
    }
    assert_eq!(paths.db, tmp.path().join("atlas.db"));
    assert_eq!(paths.unsorted, tmp.path().join("plans").join("unsorted"));
    paths.ensure().unwrap();
    assert!(paths.unsorted.is_dir());
}

#[test]
fn config_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(tmp.path().to_path_buf());
    let cfg = Config::load(&paths);
    assert_eq!(cfg.port, 7420);
    assert_eq!(cfg.locale, "en");
    assert!(!cfg.setup_done);
    let changed = Config {
        port: 7425,
        locale: "vi".into(),
        setup_done: true,
    };
    changed.save(&paths).unwrap();
    assert_eq!(Config::load(&paths), changed);
}

#[test]
fn taxonomy_default_written_and_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(tmp.path().to_path_buf());
    let tax = Taxonomy::load(&paths);
    assert!(paths.taxonomy.is_file());
    assert!(tax.contains("tech/jwt"));
    assert!(!tax.contains("tech/nope"));
    for (name, info) in &tax.tags {
        assert!(
            ["type", "domain", "tech"].contains(&info.group.as_str()),
            "{name}"
        );
        assert!(name.starts_with(&format!("{}/", info.group)), "{name}");
        for s in [&info.label_en, &info.label_vi, &info.desc_en, &info.desc_vi] {
            assert!(!s.is_empty(), "{name}");
        }
    }
    assert_eq!(tax.tags["tech/jwt"].label_vi, "JWT – token đăng nhập");
}

#[test]
fn broken_config_falls_back_to_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(tmp.path().to_path_buf());
    std::fs::write(&paths.config, "port = [not toml").unwrap();
    assert_eq!(Config::load(&paths), Config::default());
    assert_eq!(
        std::fs::read_to_string(&paths.config).unwrap(),
        "port = [not toml"
    );
}

#[test]
fn broken_taxonomy_falls_back_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(tmp.path().to_path_buf());
    std::fs::write(&paths.taxonomy, "[tags.\"x\"\nbroken").unwrap();
    let tax = Taxonomy::load(&paths);
    assert!(tax.contains("tech/jwt"));
    assert_eq!(
        std::fs::read_to_string(&paths.taxonomy).unwrap(),
        "[tags.\"x\"\nbroken"
    );
}
