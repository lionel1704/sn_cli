// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

#[macro_use]
extern crate duct;

use anyhow::{anyhow, Result};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use sn_cmd_test_utilities::util::{
    create_nrs_link, get_random_nrs_string, mk_emptyfolder, parse_files_container_output,
    parse_files_put_or_sync_output, parse_files_tree_output, safe_cmd_stderr, safe_cmd_stdout,
    safeurl_from, test_symlinks_are_valid, upload_test_symlinks_folder,
    upload_testfolder_no_trailing_slash, upload_testfolder_trailing_slash, CLI, SAFE_PROTOCOL,
};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{prelude::*, Seek, SeekFrom},
    process::Command,
};

const PRETTY_FILES_CREATION_RESPONSE: &str = "FilesContainer created at: ";
const TEST_FILE: &str = "./testdata/test.md";
const TEST_FILE_RANDOM_CONTENT: &str = "test_file_random_content.txt";
const TEST_FOLDER: &str = "./testdata/";
const TEST_FOLDER_NO_TRAILING_SLASH: &str = "./testdata";
const TEST_FOLDER_SUBFOLDER: &str = "./testdata/subfolder/";

const EXPECT_TESTDATA_PUT_CNT: usize = 11; // 8 files, plus 3 directories

#[test]
fn calling_safe_files_put_pretty() -> Result<()> {
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["files", "put", TEST_FILE])
        .assert()
        .stdout(predicate::str::contains(PRETTY_FILES_CREATION_RESPONSE))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(2))
        .stdout(predicate::str::contains(TEST_FILE).count(1))
        .success();
    Ok(())
}

#[test]
fn calling_safe_files_put() -> Result<()> {
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["files", "put", TEST_FILE, "--json"])
        .assert()
        .stdout(predicate::str::contains(PRETTY_FILES_CREATION_RESPONSE).count(0))
        .stdout(predicate::str::contains(SAFE_PROTOCOL).count(2))
        .stdout(predicate::str::contains(TEST_FILE).count(1))
        .success();
    Ok(())
}

#[test]
fn calling_safe_files_put_dry_run() -> Result<()> {
    let random_content: String = (0..10).map(|_| rand::random::<char>()).collect();
    fs::write(TEST_FILE_RANDOM_CONTENT, random_content).map_err(|e| anyhow!(e.to_string()))?;

    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FILE_RANDOM_CONTENT,
        "--json",
        "--dry-run"
    )
    .read()?;

    let (_container_xorurl, map) = parse_files_put_or_sync_output(&content);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &map[TEST_FILE_RANDOM_CONTENT].1])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive() -> Result<()> {
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["files", "put", TEST_FOLDER, "--recursive", "--json"])
        .assert()
        .stdout(predicate::str::contains(r#"+"#).count(EXPECT_TESTDATA_PUT_CNT))
        .stdout(predicate::str::contains("./testdata/test.md").count(1))
        .stdout(predicate::str::contains("./testdata/another.md").count(1))
        .stdout(predicate::str::contains("./testdata/subfolder/subexists.md").count(1))
        .success();
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive_and_set_dest_path() -> Result<()> {
    let files_container = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "/aha",
        "--recursive",
    )
    .read()?;

    let mut lines = files_container.lines();
    let files_container_xor_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not fetch next line".to_string()))?;
    let files_container_xor =
        &files_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/aha/test.md");
    let file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(file_cat, "hello tests!");

    safeurl.set_path("/aha/subfolder/subexists.md");
    let subfile_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(subfile_cat, "hello from a subfolder!");
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive_subfolder() -> Result<()> {
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec![
        "files",
        "put",
        TEST_FOLDER_SUBFOLDER,
        "--recursive",
        "--json",
    ])
    .assert()
    .stdout(predicate::str::contains(SAFE_PROTOCOL).count(3))
    .stdout(predicate::str::contains("./testdata/test.md").count(0))
    .stdout(predicate::str::contains("./testdata/another.md").count(0))
    .stdout(predicate::str::contains("./testdata/subfolder/subexists.md").count(1))
    .success();
    Ok(())
}

#[test]
fn calling_safe_files_put_emptyfolder() -> Result<()> {
    let emptyfolder_paths = mk_emptyfolder("emptyfolder").map_err(|e| anyhow!(e.to_string()))?;

    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec![
        "files",
        "put",
        &emptyfolder_paths.1,
        "--recursive",
        "--json",
    ])
    .assert()
    .stdout(predicate::str::contains(SAFE_PROTOCOL).count(1))
    .stdout(predicate::str::contains("./testdata/emptyfolder/").count(0))
    .success();

    // cleanup
    fs::remove_dir_all(&emptyfolder_paths.0).map_err(|e| anyhow!(e.to_string()))?;
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive_with_slash() -> Result<()> {
    let files_container = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive"
    )
    .read()?;

    let mut lines = files_container.lines();
    let files_container_xor_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not fetch next line".to_string()))?;
    let files_container_xor =
        &files_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/test.md");
    let file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(file_cat, "hello tests!");

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/subfolder/subexists.md");
    let subfile_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(subfile_cat, "hello from a subfolder!");
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive_without_slash() -> Result<()> {
    let files_container = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER_NO_TRAILING_SLASH,
        "--recursive"
    )
    .read()?;

    let mut lines = files_container.lines();
    let files_container_xor_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not fetch next line".to_string()))?;
    let files_container_xor =
        &files_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/testdata/test.md");
    let file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(file_cat, "hello tests!");

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/testdata/subfolder/subexists.md");
    let subfile_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(subfile_cat, "hello from a subfolder!");
    Ok(())
}

#[test]
fn calling_safe_files_sync() -> Result<()> {
    let files_container = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive"
    )
    .read()?;

    let mut lines = files_container.lines();
    let files_container_xor_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not fetch next line".to_string()))?;
    let files_container_xor =
        &files_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        TEST_FOLDER_SUBFOLDER,
        files_container_xor,
        "--recursive"
    )
    .read()?;

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/subexists.md");
    safeurl.set_content_version(Some(1));
    let synced_file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(synced_file_cat, "hello from a subfolder!");
    Ok(())
}

#[test]
fn calling_safe_files_sync_dry_run() -> Result<()> {
    let content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--json"
    )
    .read()?;

    let (container_xorurl, _) = parse_files_put_or_sync_output(&content);
    let mut target = safeurl_from(&container_xorurl)?;
    target.set_content_version(None);

    let random_content: String = (0..10).map(|_| rand::random::<char>()).collect();
    fs::write(TEST_FILE_RANDOM_CONTENT, random_content).map_err(|e| anyhow!(e.to_string()))?;
    let sync_content = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        TEST_FILE_RANDOM_CONTENT,
        target.to_string(),
        "--json",
        "--dry-run"
    )
    .read()?;

    let (_, map) = parse_files_put_or_sync_output(&sync_content);
    let mut cmd = Command::cargo_bin(CLI).map_err(|e| anyhow!(e.to_string()))?;
    cmd.args(&vec!["cat", &map[TEST_FILE_RANDOM_CONTENT].1])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn calling_safe_files_removed_sync() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let emptyfolder_paths = mk_emptyfolder("emptyfolder").map_err(|e| anyhow!(e.to_string()))?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    // let's first try with --dry-run and they should not be removed
    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let files_container_no_version = safeurl.to_string();
    let sync_cmd_output_dry_run = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        &emptyfolder_paths.1, // rather than removing the files we pass an empty folder path
        &files_container_no_version,
        "--recursive",
        "--delete",
        "--dry-run",
        "--json",
    )
    .read()?;

    safeurl.set_content_version(Some(1));
    let files_container_v1 = safeurl.to_string();
    let (target, processed_files) = parse_files_put_or_sync_output(&sync_cmd_output_dry_run);
    assert_eq!(target, files_container_v1);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    let synced_file_cat = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &files_container_xor,
        "--json"
    )
    .read()?;
    let (xorurl, files_map) = parse_files_container_output(&synced_file_cat);
    assert_eq!(xorurl, files_container_xor);
    assert_eq!(files_map.len(), EXPECT_TESTDATA_PUT_CNT);

    // Now, let's try without --dry-run and they should be effectively removed
    let sync_cmd_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        &emptyfolder_paths.1, // rather than removing the files we pass an empty folder path
        &files_container_no_version,
        "--recursive",
        "--delete",
        "--json",
    )
    .read()?;

    // cleanup
    fs::remove_dir_all(&emptyfolder_paths.0).map_err(|e| anyhow!(e.to_string()))?;

    let (target, processed_files) = parse_files_put_or_sync_output(&sync_cmd_output);
    assert_eq!(target, files_container_v1);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    // now all file items should be gone
    safeurl.set_content_version(None);
    let synced_file_cat = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &safeurl.to_string(),
        "--json"
    )
    .read()?;

    let (xorurl, files_map) = parse_files_container_output(&synced_file_cat);
    assert_eq!(xorurl, safeurl.to_string());
    assert_eq!(files_map.len(), 0);
    Ok(())
}

#[test]
fn calling_safe_files_put_recursive_with_slash_then_sync_after_modifications() -> Result<()> {
    let files_container = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER_SUBFOLDER,
        "--recursive"
    )
    .read()?;

    let file_to_delete = format!("{}/sub2.md", TEST_FOLDER_SUBFOLDER);
    let file_to_modify = format!("{}/subexists.md", TEST_FOLDER_SUBFOLDER);

    let mut lines = files_container.lines();
    let files_container_xor_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not fetch next line".to_string()))?;
    let files_container_xor =
        &files_container_xor_line[PRETTY_FILES_CREATION_RESPONSE.len()..].replace("\"", "");

    //modify file
    let file_to_modify_write = OpenOptions::new()
        .append(true)
        .open(&file_to_modify)
        .map_err(|e| anyhow!(e.to_string()))?;

    if let Err(e) = writeln!(&file_to_modify_write, " with more text!") {
        eprintln!("Couldn't write to file: {}", e);
    }

    //remove another
    fs::remove_file(&file_to_delete).map_err(|e| anyhow!(e.to_string()))?;

    // now sync
    let files_sync_result = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        TEST_FOLDER_SUBFOLDER,
        files_container_xor,
        "--recursive",
        // "--delete"
    )
    .read()?;

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_path("/subexists.md");
    safeurl.set_content_version(None);
    let file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;

    // remove modified lines
    let mut replace_test_md = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&file_to_modify)
        .map_err(|e| anyhow!(e.to_string()))?;

    replace_test_md
        .seek(SeekFrom::Start(0))
        .map_err(|e| anyhow!(e.to_string()))?;
    replace_test_md
        .write_all(b"hello from a subfolder!")
        .map_err(|e| anyhow!(e.to_string()))?;

    // readd the removed missing file
    let mut readd_missing_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&file_to_delete)
        .map_err(|e| anyhow!(e.to_string()))?;

    readd_missing_file
        .seek(SeekFrom::Start(0))
        .map_err(|e| anyhow!(e.to_string()))?;
    readd_missing_file
        .write_all(b"sub2")
        .map_err(|e| anyhow!(e.to_string()))?;

    // and now the tests...
    assert_eq!(file_cat, "hello from a subfolder! with more text!");
    assert!(files_sync_result.contains('*'));
    assert!(!files_sync_result.contains('+'));
    Ok(())
}

#[test]
fn calling_files_sync_and_fetch_with_version() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let emptyfolder_paths = mk_emptyfolder("emptyfolder").map_err(|e| anyhow!(e.to_string()))?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let files_container_no_version = safeurl.to_string();
    let sync_cmd_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        &emptyfolder_paths.1, // rather than removing the files we pass an empty folder path
        &files_container_no_version,
        "--recursive",
        "--delete",
        "--json",
    )
    .read()?;

    // cleanup
    fs::remove_dir_all(&emptyfolder_paths.0).map_err(|e| anyhow!(e.to_string()))?;

    safeurl.set_content_version(Some(1));
    let files_container_v1 = safeurl.to_string();
    let (target, processed_files) = parse_files_put_or_sync_output(&sync_cmd_output);
    assert_eq!(target, files_container_v1);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    // now all file items should be gone in version 1 of the FilesContainer
    let cat_container_v1 = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &files_container_v1,
        "--json"
    )
    .read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_container_v1);
    assert_eq!(xorurl, files_container_v1);
    assert_eq!(files_map.len(), 0);

    // but in version 0 of the FilesContainer all files should still be there
    safeurl.set_content_version(Some(0));
    let files_container_v0 = safeurl.to_string();
    let cat_container_v0 = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &files_container_v0,
        "--json"
    )
    .read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_container_v0);
    assert_eq!(xorurl, files_container_v0);
    assert_eq!(files_map.len(), EXPECT_TESTDATA_PUT_CNT);
    Ok(())
}

#[test]
fn calling_files_sync_and_fetch_with_nrsurl_and_nrs_update() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(Some(0));
    let files_container_v0 = &safeurl.to_string();
    let nrsurl = format!("safe://{}", get_random_nrs_string());
    let nrsurl_v1 = format!("{}?v=1", nrsurl);

    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &nrsurl,
        "-l",
        &files_container_v0,
    )
    .read()?;

    let emptyfolder_paths = mk_emptyfolder("emptyfolder").map_err(|e| anyhow!(e.to_string()))?;

    let sync_cmd_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        &emptyfolder_paths.1, // rather than removing the files we pass an empty folder path
        &nrsurl,
        "--recursive",
        "--delete",
        "--json",
        "--update-nrs"
    )
    .read()?;

    // cleanup
    fs::remove_dir_all(&emptyfolder_paths.0).map_err(|e| anyhow!(e.to_string()))?;

    println!("{}", sync_cmd_output);
    let (target, processed_files) = parse_files_put_or_sync_output(&sync_cmd_output);
    assert_eq!(target, nrsurl_v1);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    // now everything should be gone in version 1
    // since NRS name was updated to link version 1 of the FilesContainer
    let cat_nrsurl_v1 = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrsurl, "--json").read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_nrsurl_v1);
    assert_eq!(xorurl, nrsurl);
    assert_eq!(files_map.len(), 0);

    // but in version 0 of the NRS name it should still link to version 0 of the FilesContainer
    // where all files should still be there
    let nrsurl_v0 = format!("{}?v=0", nrsurl);
    let cat_nrsurl_v0 = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrsurl_v0, "--json").read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_nrsurl_v0);
    assert_eq!(xorurl, nrsurl_v0);
    assert_eq!(files_map.len(), EXPECT_TESTDATA_PUT_CNT);
    Ok(())
}

#[test]
fn calling_files_sync_and_fetch_without_nrs_update() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);
    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(Some(0));
    let files_container_v0 = safeurl.to_string();
    let nrsurl = format!("safe://{}", get_random_nrs_string());
    let nrsurl_v1 = format!("{}?v=1", nrsurl);

    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "nrs",
        "create",
        &nrsurl,
        "-l",
        &files_container_v0,
    )
    .read()?;

    let emptyfolder_paths = mk_emptyfolder("emptyfolder").map_err(|e| anyhow!(e.to_string()))?;

    let sync_cmd_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "sync",
        emptyfolder_paths.1, // rather than removing the files we pass an empty folder path
        &nrsurl,
        "--recursive",
        "--delete",
        "--json",
    )
    .read()?;

    // cleanup
    fs::remove_dir_all(&emptyfolder_paths.0).map_err(|e| anyhow!(e.to_string()))?;

    let (target, processed_files) = parse_files_put_or_sync_output(&sync_cmd_output);
    assert_eq!(target, nrsurl_v1);
    assert_eq!(processed_files.len(), EXPECT_TESTDATA_PUT_CNT);

    // now all file items should be gone in version 1 of the FilesContainer
    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(Some(1));
    let files_container_v1 = safeurl.to_string();
    let cat_container_v1 = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "cat",
        &files_container_v1,
        "--json"
    )
    .read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_container_v1);
    assert_eq!(xorurl, files_container_v1);
    assert_eq!(files_map.len(), 0);

    // but the NRS name should still link to version 0 of the FilesContainer
    // where all files should still be there
    let cat_nrsurl = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", &nrsurl, "--json").read()?;
    let (xorurl, files_map) = parse_files_container_output(&cat_nrsurl);
    assert_eq!(xorurl, nrsurl);
    assert_eq!(files_map.len(), EXPECT_TESTDATA_PUT_CNT);
    Ok(())
}

#[test]
fn calling_safe_files_add() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json",
    )
    .read()?;

    let (files_container_xor, _processed_files) =
        parse_files_put_or_sync_output(&files_container_output);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "add",
        TEST_FILE,
        &format!("{}/new_test.md", safeurl),
    )
    .read()?;

    safeurl.set_path("/new_test.md");
    let synced_file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(synced_file_cat, "hello tests!");
    Ok(())
}

#[test]
fn calling_safe_files_add_dry_run() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json",
    )
    .read()?;

    let (files_container_xor, _) = parse_files_put_or_sync_output(&files_container_output);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "add",
        TEST_FILE,
        &format!("{}/new_test.md", safeurl),
        "--dry-run"
    )
    .read()?;

    safeurl.set_path("/new_test.md");
    let mut cmd = Command::cargo_bin(CLI)?;
    cmd.args(&vec!["cat", &safeurl.to_string()])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn calling_safe_files_add_a_url() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    safeurl.set_path("/new_test.md");
    let _ = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "add",
        &processed_files[TEST_FILE].1,
        &safeurl.to_string(),
        "--json"
    )
    .read()?;

    let synced_file_cat = cmd!(env!("CARGO_BIN_EXE_safe"), "cat", safeurl.to_string()).read()?;
    assert_eq!(synced_file_cat, "hello tests!");
    Ok(())
}

#[test]
fn calling_files_ls() -> Result<()> {
    let files_container_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "put",
        TEST_FOLDER,
        "--recursive",
        "--json"
    )
    .read()?;

    let (files_container_xor, processed_files) =
        parse_files_put_or_sync_output(&files_container_output);

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let container_xorurl_no_version = safeurl.to_string();

    let files_ls_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "ls",
        &container_xorurl_no_version,
        "--json"
    )
    .read()?;

    // Sample output:
    //
    // Files of FilesContainer (version 0) at "safe://<xorurl>":
    // Files: 4   Size: 41   Total Files: 8   Total Size: 80
    // SIZE  CREATED               MODIFIED              NAME
    // 23    2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  .hidden.txt
    // 12    2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  .subhidden/
    // 6     2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  another.md
    // 0     2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  emptyfolder/
    // 0     2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  noextension
    // 27    2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  subfolder/
    // 12    2020-05-20T19:55:26Z  2020-05-20T19:55:26Z  test.md

    let (xorurl, files_map) = parse_files_container_output(&files_ls_output);
    assert_eq!(xorurl, container_xorurl_no_version);
    assert_eq!(files_map.len(), 7);
    assert_eq!(
        files_map[".hidden.txt"]["link"],
        processed_files[&format!("{}.hidden.txt", TEST_FOLDER)].1
    );
    assert_eq!(
        files_map["another.md"]["link"],
        processed_files[&format!("{}another.md", TEST_FOLDER)].1
    );
    assert_eq!(
        files_map["noextension"]["link"],
        processed_files[&format!("{}noextension", TEST_FOLDER)].1
    );
    assert_eq!(
        files_map["test.md"]["link"],
        processed_files[&format!("{}test.md", TEST_FOLDER)].1
    );

    assert_eq!(files_map["subfolder/"]["size"], "27");
    safeurl.set_path("subfolder");
    let subfolder_path = safeurl.to_string();
    assert_eq!(files_map["subfolder/"]["link"], subfolder_path);

    // now listing subfolder should show less files
    let files_ls_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "ls",
        &subfolder_path,
        "--json"
    )
    .read()?;

    let (xorurl, files_map) = parse_files_container_output(&files_ls_output);
    assert_eq!(xorurl, subfolder_path);
    assert_eq!(files_map.len(), 2);
    assert_eq!(
        files_map["sub2.md"]["link"],
        processed_files[&format!("{}sub2.md", TEST_FOLDER_SUBFOLDER)].1
    );
    assert_eq!(files_map["sub2.md"]["size"], "4");
    assert_eq!(
        files_map["subexists.md"]["link"],
        processed_files[&format!("{}subexists.md", TEST_FOLDER_SUBFOLDER)].1
    );
    assert_eq!(files_map["subexists.md"]["size"], "23");
    Ok(())
}

// Test:  safe ls safe://<xorurl>/subfold
//
//    note: URL path is invalid.
//
//    expected result:
//       a. exit code = 1
//       b. stderr contains "No data found for path"
#[test]
fn calling_files_ls_with_invalid_path() -> Result<()> {
    let (files_container_xor, _processed_files) = upload_testfolder_trailing_slash()?;
    let mut safeurl = safeurl_from(&files_container_xor).map_err(|e| anyhow!(e.to_string()))?;

    // set invalid path
    safeurl.set_path("subfold");
    let partial_path = safeurl.to_string();

    let args = ["files", "ls", &partial_path, "--json"];
    let stderr = safe_cmd_stderr(&args, Some(1)).map_err(|e| anyhow!(e.to_string()))?;

    assert!(stderr.contains("No data found for path"));

    Ok(())
}

// Test:  safe ls safe://<xorurl>/subfolder/sub2.md
//
//    expected result: We find the single file requested
#[test]
fn calling_files_ls_on_single_file() -> Result<()> {
    let (files_container_xor, _processed_files) = upload_testfolder_trailing_slash()?;

    let mut safeurl = safeurl_from(&files_container_xor).map_err(|e| anyhow!(e.to_string()))?;
    safeurl.set_path("/subfolder/sub2.md");
    let single_file_url = safeurl.to_string();

    let files_ls_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "ls",
        &single_file_url,
        "--json"
    )
    .read()?;

    let (_xorurl, files_map) = parse_files_container_output(&files_ls_output);
    assert_eq!(files_map.len(), 1);
    assert_eq!(files_map["sub2.md"]["size"], "4");

    Ok(())
}

// Test:  safe ls safe://<nrsname>/subfolder
//
//    safe://<nrsname> links to safe://<xorurl>/testdata
//
//    expected result: We find the 2 files beneath testdata/subfolder
#[test]
fn calling_files_ls_on_nrs_with_path() -> Result<()> {
    let (files_container_xor, _processed_files) = upload_testfolder_no_trailing_slash()?;

    let mut safeurl = safeurl_from(&files_container_xor).map_err(|e| anyhow!(e.to_string()))?;
    safeurl.set_content_version(Some(0));
    safeurl.set_path("/testdata");
    let container_xorurl_v0 = safeurl.to_string();

    let container_nrsurl = create_nrs_link(&get_random_nrs_string(), &container_xorurl_v0)?;

    let mut nrsurl_encoder = safeurl_from(&container_nrsurl).map_err(|e| anyhow!(e.to_string()))?;
    nrsurl_encoder.set_path("/subfolder");
    let nrsurl = nrsurl_encoder.to_string();

    let files_ls_output =
        cmd!(env!("CARGO_BIN_EXE_safe"), "files", "ls", &nrsurl, "--json").read()?;

    let (_xorurl, files_map) = parse_files_container_output(&files_ls_output);
    assert_eq!(files_map.len(), 2);
    assert_eq!(files_map["sub2.md"]["size"], "4");

    Ok(())
}

// Test:  safe files ls <src> --json
//    src is symlinks_test dir, put with trailing slash.
//
//    expected result: result contains 9 FileItem and filenames match.
//                     those in ./test_symlinks
#[test]
fn calling_files_ls_with_symlinks() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid().map_err(|e| anyhow!(e.to_string()))? {
        return Ok(());
    }

    let (files_container_xor, ..) =
        upload_test_symlinks_folder(true).map_err(|e| anyhow!(e.to_string()))?;

    let args = ["files", "ls", &files_container_xor, "--json"];
    let files_ls_output = safe_cmd_stdout(&args, Some(0)).map_err(|e| anyhow!(e.to_string()))?;

    // Sample output:
    //
    // Files of FilesContainer (version 0) at "safe://hnyynyss1e1ihdzuspegnqft1y5tocd5o7qgfbmmcgjdizg49bdg68ysqgbnc":
    // Files: 11   Size: 520   Total Files: 20   Total Size: 564
    // SIZE  CREATED               MODIFIED              NAME
    // 391   2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  absolute_links.txt
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  broken_rel_link.txt -> non-existing-target
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  dir_link -> sub
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  dir_link_deep -> sub/deep
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  dir_link_link -> dir_link
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  dir_outside -> ../
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  file_link -> realfile.txt
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  file_link_link -> file_link
    // 0     2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  file_outside -> ../file_outside
    // 21    2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  realfile.txt
    // 34    2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  sub/
    // 10    2020-06-11T22:13:36Z  2020-06-11T22:13:36Z  sub2/

    let (xorurl, files_map) = parse_files_container_output(&files_ls_output);
    assert_eq!(xorurl, files_container_xor);
    assert_eq!(files_map.len(), 12);
    assert!(files_map.contains_key("absolute_links.txt"));
    assert!(files_map.contains_key("broken_rel_link.txt"));
    assert!(files_map.contains_key("file_link"));
    assert!(files_map.contains_key("file_link_link"));
    assert!(files_map.contains_key("dir_link"));
    assert!(files_map.contains_key("realfile.txt"));
    assert!(files_map.contains_key("sub/"));

    // todo:
    // 1. test ls'ing an individual symlink

    Ok(())
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn calling_files_tree() -> Result<()> {
    let (files_container_xor, _processed_files) =
        upload_testfolder_trailing_slash().map_err(|e| anyhow!(e.to_string()))?;

    let mut safeurl = safeurl_from(&files_container_xor)?;
    safeurl.set_content_version(None);
    let container_xorurl_no_version = safeurl.to_string();

    let files_tree_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "tree",
        &container_xorurl_no_version,
        "--json"
    )
    .read()?;

    let root = parse_files_tree_output(&files_tree_output);
    assert_eq!(root["name"], container_xorurl_no_version);
    assert_eq!(root["sub"].as_array().unwrap().len(), 7);
    assert_eq!(root["sub"][0]["name"], ".hidden.txt");
    assert_eq!(root["sub"][1]["name"], ".subhidden");
    assert_eq!(root["sub"][1]["sub"][0]["name"], "test.md");
    assert_eq!(root["sub"][2]["name"], "another.md");
    assert_eq!(root["sub"][3]["name"], "emptyfolder");
    assert_eq!(root["sub"][3]["sub"][0]["name"], ".gitkeep");
    assert_eq!(root["sub"][4]["name"], "noextension");
    assert_eq!(root["sub"][5]["name"], "subfolder");
    assert_eq!(root["sub"][5]["sub"][0]["name"], "sub2.md");
    assert_eq!(root["sub"][5]["sub"][1]["name"], "subexists.md");
    assert_eq!(root["sub"][6]["name"], "test.md");

    let files_tree_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "tree",
        &container_xorurl_no_version
    )
    .read()?;

    let should_match = format!(
        "{}\n{}",
        container_xorurl_no_version,
        "\
├── .hidden.txt
├── .subhidden
│   └── test.md
├── another.md
├── emptyfolder
│   └── .gitkeep
├── noextension
├── subfolder
│   ├── sub2.md
│   └── subexists.md
└── test.md

3 directories, 8 files"
    );
    assert_eq!(files_tree_output, should_match);

    let files_tree_output = cmd!(
        env!("CARGO_BIN_EXE_safe"),
        "files",
        "tree",
        &container_xorurl_no_version,
        "--details",
        "--json",
    )
    .read()?;

    let root = parse_files_tree_output(&files_tree_output);
    assert_eq!(root["name"], container_xorurl_no_version);
    assert_eq!(root["sub"].as_array().unwrap().len(), 7);
    assert_eq!(root["sub"][0]["name"], ".hidden.txt");
    assert_eq!(root["sub"][0]["details"]["type"], "text/plain");
    assert_eq!(root["sub"][1]["name"], ".subhidden");
    assert_eq!(root["sub"][1]["details"]["type"], "inode/directory");
    assert_eq!(root["sub"][1]["sub"][0]["name"], "test.md");
    assert_eq!(root["sub"][2]["name"], "another.md");
    assert_eq!(root["sub"][2]["details"]["size"], "6");
    assert_eq!(root["sub"][2]["details"]["type"], "text/markdown");
    assert_eq!(root["sub"][3]["name"], "emptyfolder");
    assert_eq!(root["sub"][3]["details"]["size"], "0");
    assert_eq!(root["sub"][3]["details"]["type"], "inode/directory");
    assert_eq!(root["sub"][4]["name"], "noextension");
    assert_eq!(root["sub"][4]["details"]["size"], "0");
    assert_eq!(root["sub"][4]["details"]["type"], "Raw");
    assert_eq!(root["sub"][5]["name"], "subfolder");
    assert_eq!(root["sub"][5]["sub"][0]["name"], "sub2.md");
    assert_eq!(root["sub"][5]["sub"][1]["name"], "subexists.md");
    assert_eq!(root["sub"][6]["name"], "test.md");
    Ok(())
}

// Test:  safe files tree <src>
//    src is symlinks_test dir, put with trailing slash.
//
//    expected result: output matches output of `tree ./test_symlinks`
#[test]
fn calling_files_tree_with_symlinks() -> Result<()> {
    // Bail if test_symlinks not valid. Typically indicates missing perms on windows.
    if !test_symlinks_are_valid()? {
        return Ok(());
    }

    let (files_container_xor, ..) = upload_test_symlinks_folder(true)?;

    let args = ["files", "tree", &files_container_xor];
    let stdout = safe_cmd_stdout(&args, Some(0))?;

    // note: this is output from `tree` command on linux.
    // `files tree` output should match exactly.
    let should_match = format!(
        "{}\n{}",
        files_container_xor,
        "\
├── absolute_links.txt
├── broken_rel_link.txt -> non-existing-target
├── dir_link -> sub
├── dir_link_deep -> sub/deep
├── dir_link_link -> dir_link
├── dir_outside -> ../
├── file_link -> realfile.txt
├── file_link_link -> file_link
├── file_outside -> ../file_outside
├── realfile.txt
├── sub
│   ├── deep
│   │   └── a_file.txt
│   ├── infinite_loop -> infinite_loop
│   ├── parent_dir -> ..
│   ├── parent_dir_file_link.txt -> ../realfile.txt
│   ├── readme.md
│   ├── sibling_dir -> ../sub2
│   ├── sibling_dir_file.md -> ../sub2/hello.md
│   └── sibling_dir_trailing_slash -> ../sub2/
└── sub2
    ├── hello.md
    └── sub2 -> ../sub2

11 directories, 12 files
"
    );
    assert_eq!(stdout, should_match);

    Ok(())
}
