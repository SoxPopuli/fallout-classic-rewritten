use std::{
    io::{stdin, Read, Cursor},
};

use pal::PalFile;

fn main() {

    let mut data = vec![];
    stdin().read_to_end(&mut data).unwrap();

    let pal = PalFile::open(&mut Cursor::new(data));

    /*
    for c in pal.colors.iter() {
        println!("r: {}, g: {}, b: {}", c.red, c.green, c.blue);
    }
    */

    print_html(&pal.unwrap());
}

fn print_html(pal: &PalFile) {
    println!("<head>");
    println!("<style>");
    println!(".text {{ color: white; background-color: black }}");
    println!("</style>");
    println!("</head>");


    println!("<body style=\"background-color: black\">");

    for c in pal.colors.iter() {
        let style_string = format!("style=\"\
            background-color: rgb({}, {}, {})
        \"", c.red, c.green, c.blue);

        let text = format!("{}, {}, {}", c.red, c.green, c.blue);
        let text_elem = format!("<span class=\"text\">{}</span>", text);

        println!("<p {}>{}</p>", style_string, text_elem);
    }

    println!("</body>");
}
