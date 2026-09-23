use island_plugin_api::{
    FramePacket, HostToPlugin, PluginManifest, SurfaceFrame, read_message, validate_plugin_id,
    write_frame, write_json,
};
use std::io::{Cursor, ErrorKind};

#[test]
fn concatenated_json_and_frame_round_trip() {
    let mut bytes = Vec::new();
    write_json(&mut bytes, &HostToPlugin::Shutdown).unwrap();
    write_frame(
        &mut bytes,
        &SurfaceFrame {
            width: 2,
            height: 1,
            frame_id: u64::MAX,
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
        },
    )
    .unwrap();
    let mut reader = Cursor::new(bytes);
    let FramePacket::Json(json) = read_message(&mut reader).unwrap() else {
        panic!("expected JSON")
    };
    assert!(matches!(
        serde_json::from_slice::<HostToPlugin>(&json).unwrap(),
        HostToPlugin::Shutdown
    ));
    let FramePacket::Frame(frame) = read_message(&mut reader).unwrap() else {
        panic!("expected frame")
    };
    assert_eq!(frame.frame_id, u64::MAX);
    assert_eq!(frame.pixels, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(
        read_message(&mut reader).unwrap_err().kind(),
        ErrorKind::UnexpectedEof
    );
}

#[test]
fn malformed_packets_fail_without_allocating_declared_oversize() {
    for bytes in [
        vec![0, 0, 0, 0],
        u32::MAX.to_le_bytes().to_vec(),
        vec![1, 0, 0, 0, 99],
        vec![2, 0, 0, 0, 2, 0],
    ] {
        assert_eq!(
            read_message(&mut Cursor::new(bytes)).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
    assert_eq!(
        read_message(&mut Cursor::new(vec![2, 0, 0, 0, 1]))
            .unwrap_err()
            .kind(),
        ErrorKind::UnexpectedEof
    );
}

#[test]
fn invalid_frames_never_write_partial_packets() {
    for (width, height, pixels) in [(0, 1, vec![]), (1, 1, vec![0; 3]), (u32::MAX, 1, vec![])] {
        let mut bytes = Vec::new();
        assert!(
            write_frame(
                &mut bytes,
                &SurfaceFrame {
                    width,
                    height,
                    pixels,
                    frame_id: 0
                }
            )
            .is_err()
        );
        assert!(bytes.is_empty());
    }
}

#[test]
fn ids_and_entry_paths_cannot_escape_package_root() {
    for id in [
        "",
        ".",
        "..",
        "../victim",
        "/tmp/victim",
        "com..bad",
        "com/bad",
        "C:\\bad",
        "builtin.music",
    ] {
        assert!(validate_plugin_id(id).is_err(), "{id}");
    }
    assert!(validate_plugin_id("com.example.my-plugin_2").is_ok());
    let base = include_str!("fixtures/plugin.toml");
    let mut manifest = PluginManifest::parse(base).unwrap();
    for path in [
        "../victim",
        "/tmp/bin",
        "C:/bin",
        "bin/../victim",
        "bin\\victim",
        "",
    ] {
        manifest.entry.command = Some(path.into());
        assert!(manifest.validate().is_err(), "{path}");
    }
    manifest.entry.command = Some("bin/player".into());
    assert!(manifest.validate().is_ok());
}

#[test]
fn control_messages_are_bounded_and_permissions_default_to_none() {
    let oversized = "x".repeat(64 * 1024);
    let mut wire = Vec::new();
    assert!(island_plugin_api::write_json(&mut wire, &oversized).is_err());
    assert!(wire.is_empty());
    let manifest = island_plugin_api::PluginManifest::parse("api_version=2\n[plugin]\nid='com.test.empty'\nname='Empty'\nversion='1.0.0'\n[entry]\nkind='ipc'\ncommand='plugin'\n").unwrap();
    assert_eq!(manifest.permission_set().unwrap().iter().count(), 0);
}

#[test]
fn legacy_present_still_expands_and_codex_links_cannot_carry_commands() {
    let legacy: island_plugin_api::PluginToHost =
        serde_json::from_str(r#"{"type":"present","priority":20}"#).unwrap();
    assert!(matches!(
        legacy,
        island_plugin_api::PluginToHost::Present {
            mode: island_plugin_api::PresentationMode::Expanded,
            ..
        }
    ));
    let valid = "codex://threads/01a0aacd-0c1d-7d62-bd33-91e5de624407";
    assert!(island_plugin_api::is_codex_thread_url(valid));
    for bad in [
        format!("{valid}?q=execute"),
        format!("{valid}#x"),
        "codex://threads/new".into(),
        "codex://settings".into(),
        "codex://threads/../../tmp".into(),
    ] {
        assert!(!island_plugin_api::is_codex_thread_url(&bad));
    }
}

#[test]
fn collapsed_icons_are_optional_and_strictly_bounded() {
    use island_plugin_api::PluginToHost;
    let legacy: PluginToHost =
        serde_json::from_str(r#"{"type":"set_collapsed_meta","leading":"Music"}"#).unwrap();
    assert!(matches!(
        legacy,
        PluginToHost::SetCollapsedMeta { icon: None, .. }
    ));
    let mut icon = SurfaceFrame {
        width: 72,
        height: 72,
        frame_id: 0,
        pixels: vec![255; 72 * 72 * 4],
    };
    assert!(icon.validate_icon().is_ok());
    icon.pixels.pop();
    assert!(icon.validate_icon().is_err());
    icon.width = 97;
    icon.height = 1;
    icon.pixels = vec![255; 97 * 4];
    assert!(icon.validate().is_ok());
    assert!(icon.validate_icon().is_err());
    let icon = SurfaceFrame {
        width: 96,
        height: 96,
        frame_id: 0,
        pixels: vec![255; 96 * 96 * 4],
    };
    let message = PluginToHost::SetCollapsedMeta {
        leading: None,
        trailing: Some("Needs you".into()),
        dot: None,
        icon: Some(icon.clone()),
    };
    let mut bytes = vec![];
    write_json(&mut bytes, &message).expect("largest icon must fit the unchanged control limit");
    let FramePacket::Json(body) = read_message(&mut Cursor::new(bytes)).unwrap() else {
        panic!("expected metadata");
    };
    let PluginToHost::SetCollapsedMeta {
        icon: Some(decoded),
        ..
    } = serde_json::from_slice(&body).unwrap()
    else {
        panic!("missing icon");
    };
    assert_eq!(decoded, icon);
}
#[test]
fn url_launch_replies_are_an_opt_in_extension() {
    use island_plugin_api::PluginToHost;
    let old: PluginToHost =
        serde_json::from_str(r#"{"type":"open_url","url":"https://example.com"}"#).unwrap();
    assert!(matches!(
        old,
        PluginToHost::OpenUrl {
            request_id: None,
            ..
        }
    ));
    let roundtrip = serde_json::to_value(old).unwrap();
    assert!(roundtrip.get("request_id").is_none());
}
