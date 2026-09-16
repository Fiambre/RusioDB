//! Herramienta de uso único: rasteriza `assets/cat.svg` a `assets/icon.ico`
//! en los tamaños estándar de ícono de Windows. Se corre a mano con
//! `cargo run --example gen_icon` cuando el logo cambia; el `.ico` resultante
//! se commitea y no se regenera en cada build.

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use std::fs;

const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn main() {
    let svg_data = fs::read("assets/cat.svg").expect("no se pudo leer assets/cat.svg");
    let tree = Tree::from_data(&svg_data, &Options::default()).expect("SVG inválido");
    let svg_size = tree.size();

    let mut icon_dir = IconDir::new(ResourceType::Icon);
    for size in SIZES {
        let mut pixmap = Pixmap::new(size, size).expect("tamaño de ícono inválido");
        let transform = Transform::from_scale(
            size as f32 / svg_size.width(),
            size as f32 / svg_size.height(),
        );
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let image = IconImage::from_rgba_data(size, size, pixmap.data().to_vec());
        let entry = IconDirEntry::encode(&image).expect("no se pudo codificar el tamaño de ícono");
        icon_dir.add_entry(entry);
    }

    let mut file = fs::File::create("assets/icon.ico").expect("no se pudo crear assets/icon.ico");
    icon_dir
        .write(&mut file)
        .expect("no se pudo escribir el ícono");
    println!("assets/icon.ico generado con tamaños {SIZES:?}");
}
