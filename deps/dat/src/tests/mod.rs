use crate::{
    DatFile,
    tree,
    tree::Node,
    tree::NodeType,
};

use std::io::Cursor;
use std::fs::File;

fn load_dat_f1() -> DatFile {
    let path = "../../../../reference/f1/MASTER.DAT";
    let f = File::open(path).unwrap();
    DatFile::open(f).unwrap()
}

fn load_dat_f2() -> DatFile {
    let path = "../../../../reference/f2/master.dat";
    let f = File::open(path).unwrap();
    DatFile::open(f).unwrap()
}

#[test]
fn lzss_test() {
    const PACKED_FILE: &[u8] = include_bytes!("./shady.frm.lzss");
    const UNPACKED_FILE: &[u8] = include_bytes!("./shady.frm");

    let input = Cursor::new( PACKED_FILE.to_vec() );
    let unpacked = DatFile::decompress_lzss_inner(input, UNPACKED_FILE.len()).unwrap();

    assert_eq!(unpacked.as_slice(), UNPACKED_FILE);
}

fn mock_tree() -> tree::FileTree {
    let entry = tree::FileEntry::default();
    let nodes = vec![
        Node::new_file("color.pal", "./color.pal", entry.clone()),
        Node::new_dir("art", "./art", Vec::new())
    ];

    let mut t = tree::FileTree::create(nodes).unwrap();
    t.insert("art/file1", entry.clone()).unwrap();

    return t;
}

#[test]
fn tree_test() {
    let tree = mock_tree();

    {
        let n = tree.get("./color.pal").unwrap();
        let lock = n.read().unwrap();
        assert!(matches!(&lock.node_type, NodeType::File { entry: _ }));
        assert_eq!(lock.get_name(), "color.pal");
    }

    {
        let n = tree.get(".").unwrap();
        let lock = n.read().unwrap();
        let name = lock.get_name();
        assert_eq!(name, ".");
    }

    {
        let n = tree.get("art").unwrap();
        let lock = n.read().unwrap();
        let name = lock.get_name();
        assert_eq!(name, "art");
    }

    {
        let n = tree.get("art/file1").unwrap();
        let lock = n.read().unwrap();
        assert!(matches!(&lock.node_type, NodeType::File { entry: _ }));
    }

    {
        let n = tree.get("file/that/does/not/exist");
        assert!(n.is_none());
    }
}
