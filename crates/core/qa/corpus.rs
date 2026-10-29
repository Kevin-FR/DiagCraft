//! Original synthetic quality corpus, deliberately outside the production library.
use diagcraft::{
    document::{ConnectorKind, Document, Port, ShapeKind},
    images::ImageAsset,
};

pub fn grid(columns: usize, rows: usize) -> Document {
    assert!(columns > 0 && rows > 0 && columns * rows <= 1000);
    let mut d = Document {
        title: "Synthetic grid — quality corpus".into(),
        page_size: [columns as f32 * 260. + 120., rows as f32 * 150. + 120.],
        ..Default::default()
    };
    let mut ids = Vec::new();
    for y in 0..rows {
        for x in 0..columns {
            let p = [60. + x as f32 * 260., 60. + y as f32 * 150.];
            let id = d.add_node(ShapeKind::Rounded, p);
            let n = d.node_mut(id).unwrap();
            n.position = p;
            n.size = [160., 72.];
            n.font_size = 16.;
            n.text = format!("Bloc {} · 中文", y * columns + x + 1);
            ids.push(id);
        }
    }
    for y in 0..rows {
        for x in 1..columns {
            let at = y * columns + x;
            let id = d
                .connect(ids[at - 1], ids[at], ConnectorKind::Elbow)
                .unwrap();
            let edge = d.connectors.iter_mut().find(|e| e.id == id).unwrap();
            edge.from_port = Port::Right;
            edge.to_port = Port::Left;
        }
    }
    d
}

pub fn multipage() -> Document {
    let mut d = grid(20, 10);
    d.title = "Synthetic five-page corpus".into();
    for _ in 1..5 {
        d.add_page(true).unwrap();
    }
    d
}

pub fn images() -> Document {
    let mut pixels = vec![0u8; 2000 * 2000 * 4];
    for (i, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(if i % 2000 < 1000 {
            &[20, 140, 220, 255]
        } else {
            &[240, 180, 30, 128]
        });
    }
    let asset = ImageAsset::from_rgba(2000, 2000, &pixels).unwrap();
    let mut d = Document {
        title: "Synthetic 12-megapixel image corpus".into(),
        page_size: [1200., 800.],
        ..Default::default()
    };
    for p in [[60., 60.], [600., 60.], [60., 420.]] {
        let id = d.add_image(asset.clone(), p).unwrap();
        let n = d.node_mut(id).unwrap();
        n.size = [400., 300.];
        n.image.as_mut().unwrap().crop = [0.1, 0.1, 0.8, 0.8];
    }
    d
}
