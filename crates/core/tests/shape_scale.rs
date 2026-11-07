use diagcraft::{
    document::{Document, ShapeKind},
    pages::{PageLayout, Unit},
    raster::{self, PngOptions},
    symbols, templates, text_layout,
};

#[test]
fn new_blocks_have_useful_physical_proportions_on_both_a4_orientations() {
    let mm = Unit::Millimeters;
    for page_size in [
        [mm.to_px(210.), mm.to_px(297.)],
        [mm.to_px(297.), mm.to_px(210.)],
    ] {
        let mut doc = Document {
            page_size,
            page_layout: PageLayout {
                unit: mm,
                auto_size: false,
            },
            ..Default::default()
        };
        for kind in [ShapeKind::Rectangle, ShapeKind::Rounded, ShapeKind::Ellipse] {
            let id = doc.add_node(kind, [page_size[0] / 2., page_size[1] / 2.]);
            doc.node_mut(id).unwrap().text_style.wrap = true;
            let n = doc.node(id).unwrap();
            assert_eq!(n.size, [120., 60.]);
            assert_eq!(n.font_size, 12.);
            assert!((mm.from_px(n.size[0]) - 31.75).abs() < 0.001);
            assert!((mm.from_px(n.size[1]) - 15.875).abs() < 0.001);
            // About 15% of portrait A4 width, or 11% of landscape A4 width.
            assert!((0.10..0.16).contains(&(n.size[0] / page_size[0])));
            assert!((0.05..0.08).contains(&(n.size[1] / page_size[1])));
            assert_eq!(n.font_size * 72. / 96., 9.);
            let layout = n.text_layout();
            let area = text_layout::area(n).1;
            assert!(
                layout.width <= area[0] && layout.height <= area[1],
                "default label for {kind:?} must fit"
            );
        }
        let id = doc.add_node(ShapeKind::Diamond, [page_size[0] / 2., page_size[1] / 2.]);
        doc.node_mut(id).unwrap().text_style.wrap = true;
        let diamond = doc.node(id).unwrap();
        assert_eq!(diamond.size, [160., 120.]);
        let area = text_layout::area(diamond).1;
        let layout = diamond.text_layout();
        assert!(
            layout.width <= area[0] && layout.height <= area[1],
            "diamond {:?} font{} layout{}x{} area{}x{}",
            diamond.text,
            diamond.font_size,
            layout.width,
            layout.height,
            area[0],
            area[1]
        );
        doc.validate().unwrap();
    }
}

#[test]
fn every_insertion_uses_document_pixels_independently_of_page_preset_or_units() {
    let sizes = diagcraft::pages::presets();
    for preset in &sizes[..3] {
        // A3, A4 and A5
        for unit in [Unit::Pixels, Unit::Millimeters, Unit::Inches] {
            let mut doc = Document {
                page_size: preset.size,
                page_layout: PageLayout {
                    unit,
                    auto_size: false,
                },
                ..Default::default()
            };
            for kind in symbols::all_kinds() {
                let center = [200., 200.];
                let id = doc.add_node(kind, center);
                let n = doc.node(id).unwrap();
                let palette = symbols::preset(kind);
                assert_eq!(n.size, palette.size, "shared size for {kind:?}");
                assert_eq!(
                    n.font_size, palette.font_size,
                    "shared font size for {kind:?}"
                );
                assert_eq!(n.center(), center);
                assert!(n.size.iter().all(|size| *size >= 24.));
                assert_eq!(n.font_size, if kind == ShapeKind::Text { 18. } else { 12. });
            }
            doc.validate().unwrap();
        }
    }
}

#[test]
fn special_symbol_defaults_keep_their_characteristic_geometry_and_readable_captions() {
    use ShapeKind::*;
    for (kind, size) in [
        (Star, [80., 80.]),
        (BraceLeft, [30., 90.]),
        (Lightning, [50., 80.]),
        (UmlClass, [180., 120.]),
        (Component, [140., 80.]),
        (Package, [160., 100.]),
        (Actor, [54., 90.]),
        (Lifeline, [100., 160.]),
        (Fork, [140., 24.]),
        (InitialState, [32., 32.]),
        (FinalState, [32., 32.]),
        (UseCase, [160., 90.]),
        (Frame, [300., 180.]),
        (Swimlane, [300., 180.]),
        (TitleBlock, [300., 180.]),
        (Text, [300., 48.]),
        (Document, [120., 90.]),
        (Hexagon, [120., 90.]),
        (OffPage, [150., 90.]),
        (PredefinedProcess, [120., 60.]),
        (InternalStorage, [120., 60.]),
    ] {
        let n = symbols::preset(kind);
        assert_eq!(n.size, size, "{kind:?}");
    }
    for kind in [
        Rectangle, Rounded, Ellipse, Diamond, UmlClass, Component, Package, UseCase,
    ] {
        let n = symbols::preset(kind);
        let layout = n.text_layout();
        let area = text_layout::area(&n).1;
        assert!(
            layout.width <= area[0] && layout.height <= area[1],
            "{kind:?}: layout {}x{}, area {}x{}",
            layout.width,
            layout.height,
            area[0],
            area[1]
        );
    }
    assert!(symbols::preset(Actor).text.is_empty());
    assert!(symbols::preset(Star).text.is_empty());
}

#[test]
fn saved_geometry_custom_symbols_and_explicit_templates_keep_their_sizes() {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rectangle, [350., 250.]);
    let n = doc.node_mut(id).unwrap();
    n.size = [220., 84.];
    n.font_size = 18.;
    n.text = "Saved dimensions".into();
    let old = doc.clone();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("existing.draw");
    doc.save(&path).unwrap();
    let loaded = Document::load(&path).unwrap();
    assert_eq!(loaded, old);
    assert_eq!(loaded.node(id).unwrap().size, [220., 84.]);
    let mut library = diagcraft::library::Library::default();
    library.add_symbol(doc.node(id).unwrap()).unwrap();
    assert_eq!(library.symbols[0].size, [220., 84.]);
    assert_eq!(library.symbols[0].font_size, 18.);
    let template = templates::support();
    let action = template
        .nodes
        .iter()
        .find(|n| n.text == "Décrire le besoin")
        .unwrap();
    assert_eq!(action.size, [280., 90.]);
    assert_eq!(action.position, [420., 270.]);
    assert_eq!(action.font_size, 22.);
    let diamond = template
        .nodes
        .iter()
        .find(|n| n.kind == ShapeKind::Diamond)
        .unwrap();
    assert_eq!(diamond.size, [260., 140.]);
    assert_eq!(diamond.font_size, 22.);
    let decoded: Document =
        serde_json::from_str(&serde_json::to_string(&template).unwrap()).unwrap();
    assert_eq!(decoded, template);
    assert_eq!(doc.format_version, 8);
}

#[test]
fn exports_preserve_page_dimensions_and_new_block_size_without_rescaling_the_source() {
    let page_size = diagcraft::pages::presets()[1].size;
    let mut doc = Document {
        page_size,
        ..Default::default()
    };
    let id = doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    let before = doc.clone();
    let svg = diagcraft::svg::export(&doc).unwrap();
    assert!(svg.contains(&format!(
        "width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"",
        page_size[0], page_size[1], page_size[0], page_size[1]
    )));
    assert!(svg.contains("<rect x=\"240\" y=\"270\" width=\"120\" height=\"60\""));
    for (width, height) in [(397, 561), (794, 1122)] {
        let bytes = raster::export(
            &doc,
            &PngOptions {
                width,
                height,
                ..Default::default()
            },
        )
        .unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (width, height));
    }
    assert_eq!(doc, before);
    assert_eq!(doc.node(id).unwrap().size, [120., 60.]);
}

#[test]
fn common_palette_captions_remain_readable_in_all_seven_languages() {
    let translations: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/locales/messages.json")).unwrap();
    for kind in [
        ShapeKind::Rectangle,
        ShapeKind::Rounded,
        ShapeKind::Ellipse,
        ShapeKind::Diamond,
        ShapeKind::PredefinedProcess,
        ShapeKind::UseCase,
    ] {
        let preset = symbols::preset(kind);
        let caption = &translations[&preset.text];
        for language in ["fr", "en", "de", "es", "it", "pt", "zh-Hans"] {
            let mut n = preset.clone();
            n.text = caption[language]
                .as_str()
                .expect("translated caption")
                .to_owned();
            let layout = n.text_layout();
            let area = text_layout::area(&n).1;
            assert!(
                layout.width <= area[0] && layout.height <= area[1],
                "{kind:?}, {language}, {:?}: {}x{} in {}x{}",
                n.text,
                layout.width,
                layout.height,
                area[0],
                area[1]
            );
        }
    }
}

#[test]
fn decorative_shapes_share_empty_captions_across_direct_and_palette_creation() {
    use ShapeKind::*;
    let mut doc = diagcraft::document::Document::default();
    for kind in [
        Triangle,
        RightTriangle,
        Pentagon,
        Octagon,
        Plus,
        Cross,
        Heart,
        Shield,
        Drop,
        Star,
        Star4,
        Star8,
        Gear,
        Cube,
        Cone,
        Pyramid,
        Cloud,
        BraceLeft,
        BraceRight,
        Lightning,
        Actor,
        Lifeline,
        Fork,
        InitialState,
        FinalState,
        Frame,
        Swimlane,
        TitleBlock,
    ]
    .into_iter()
    .chain(ShapeKind::ARROWS.iter().copied())
    {
        let id = doc.add_node(kind, [200., 200.]);
        assert!(doc.node(id).unwrap().text.is_empty(), "direct {kind:?}");
        assert!(symbols::preset(kind).text.is_empty(), "palette {kind:?}");
    }
    let text = doc.add_node(Text, [200., 200.]);
    assert_eq!(doc.node(text).unwrap().text, "Votre texte");
    assert_eq!(symbols::preset(Text).text, "Votre texte");
    // Explicit content remains possible on every decorative shape.
    let id = doc.add_node(Actor, [200., 200.]);
    doc.node_mut(id).unwrap().text = "Custom caption".into();
    let restored: diagcraft::document::Document =
        serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(restored.node(id).unwrap().text, "Custom caption");
}

#[test]
fn all_nonempty_palette_captions_fit_in_every_available_translation() {
    let translations: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/locales/messages.json")).unwrap();
    let mut overflow = vec![];
    for kind in symbols::all_kinds() {
        let preset = symbols::preset(kind);
        if preset.text.is_empty() {
            continue;
        }
        for language in ["fr", "en", "de", "es", "it", "pt", "zh-Hans"] {
            let mut n = preset.clone();
            n.text = translations[&preset.text][language]
                .as_str()
                .unwrap_or(&preset.text)
                .to_owned();
            let layout = n.text_layout();
            let area = text_layout::area(&n).1;
            // Standalone Text intentionally allows vertical overflow without clipping.
            if layout.width > area[0] || (kind != ShapeKind::Text && layout.height > area[1]) {
                overflow.push(format!(
                    "{kind:?}, {language}, {:?}: layout {:.2}×{:.2}, zone {:.2}×{:.2}, size {:?}",
                    n.text, layout.width, layout.height, area[0], area[1], n.size
                ));
            }
        }
    }
    assert!(
        overflow.is_empty(),
        "{} caption overflows:\n{}",
        overflow.len(),
        overflow.join("\n")
    );
}
