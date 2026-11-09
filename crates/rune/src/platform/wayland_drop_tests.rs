use std::path::PathBuf;

use super::{file_paths, hostname};

#[test]
fn a_uri_list_gives_its_local_files_decoded() {
    let list = b"# from a file manager\r\nfile:///srv/a/Report%20Final.pdf\r\nfile://localhost/tmp/x%2Fy\r\n\r\nfile:///tmp/caf%C3%A9.png";
    assert_eq!(file_paths(list), [
        PathBuf::from("/srv/a/Report Final.pdf"),
        PathBuf::from("/tmp/x/y"),
        PathBuf::from("/tmp/café.png"),
    ]);
}

#[test]
fn other_uris_and_other_hosts_are_skipped() {
    let list = b"https://example.com/a.png\nfile://elsewhere.example/tmp/a\nmailto:a@example.com\nfile:///tmp/b\n";
    assert_eq!(file_paths(list), [PathBuf::from("/tmp/b")]);
}

#[test]
fn this_machines_name_is_local() {
    let Some(host) = hostname() else { return };
    let mut list = b"file://".to_vec();
    list.extend_from_slice(&host);
    list.extend_from_slice(b"/tmp/c");
    assert_eq!(file_paths(&list), [PathBuf::from("/tmp/c")]);
}

#[test]
fn a_stray_percent_stays() {
    assert_eq!(file_paths(b"FILE:///tmp/100%/a%zz%4"), [PathBuf::from("/tmp/100%/a%zz%4")]);
}
