use std::{ 
    io::{ stdin, Read, Cursor, stdout, Write },
    collections::hash_map::Entry,
    env::{ Args, args },
    fs::File,
    path::Path,
};

use dat::{ 
    DatFile,
    tree::FileState,
    tree::Node,
};

#[derive(Debug, Default, Clone)]
struct Options {
    show_help: bool,
    unpack: bool,

    input: Option<String>,
    file: Option<String>,
    output: Option<String>,
}

impl Options {
    fn new(args: &mut Args) -> Self {
        let mut this = Self::default();
        let args: Vec<_> = args.into_iter()
            .collect();

        let mut i = 1;
        while i < args.len() {
            let arg = args[i].clone();

            match arg.as_str() {
                "--help" | "-h" => {
                    this.show_help = true;
                },
                "--extract" | "-e" => {
                    this.output = Some(args[i+1].clone());
                    i += 1;
                },
                "--file" | "-f" => {
                    this.file = Some(args[i + 1].clone());
                    i += 1;
                },
                "--unpack" | "-u" => {
                    this.unpack = true;
                },
                _ => { 
                    this.input = Some(arg);
                },
            }

            i += 1;
        }

        this
    }
}

fn print_help() {
    println!("{} [options] FILE", args().nth(0).unwrap_or("read-dat".into()));
    println!("reads fallout 1/2 dat files from stdin");
    println!();

    println!("-h --help                print help message");
    println!("-e --extract path        extract all files");
    println!("-f --file name           extract one file");
    println!("-u --unpack              decompress file");
}

fn main()
{
    let mut args = std::env::args();
    let options = Options::new(&mut args);

    if options.show_help {
        print_help();
        return;
    }

    //println!("{:?}", options.input);
    //println!("{:?}", options.output);

    let input = options.input.expect("missing input");
    let file = File::open(input).unwrap();

    let dat = DatFile::open(file).unwrap();

    if let Some(file) = options.file {
        let file = file.replace("/", "\\");
        let entry = dat.registry.get(&file).expect(&format!("file not found: {}", file));

        let entry = {
            let lock = entry.read().unwrap();
            lock.get_file_entry().unwrap().clone()
        };

        let data = match options.unpack {
            true => dat.unpack_file(&entry).unwrap(),
            false => dat.get_entry_data(&entry).unwrap(),
        };


        stdout().write_all(&data).unwrap();
    } else if let Some(out) = options.output {
        let path_prefix = Path::new(&out);

        for node in &dat.registry {
            let name = node.get_name();
            let file = node.get_file_entry().unwrap();

            let full_path = path_prefix.join(name.replace("\\", "/"));

            if let Some(dir_path) = full_path.parent() {
                std::fs::create_dir_all(dir_path).unwrap();
            }

            let mut out_file = File::create(full_path.clone()).unwrap();
            let data = dat.unpack_file(file).unwrap();
            out_file.write_all(&data).unwrap();

            println!("wrote {}", full_path.to_str().unwrap());
        }
    } else {
        let mut lines = vec![];
        for node in &dat.registry {
            let name = node.get_name();
            let path = node.get_path();
            let file = node.get_file_entry().unwrap();

            let state = match file.state {
                FileState::Uncompressed => "Uncompressed",
                FileState::Compressed { size: _ } => "Compressed",
            };

            let line = format!("{:<40}: {}", path, state);
            lines.push(line);
        }

        for l in lines { println!("{}", l); }
    }
}
