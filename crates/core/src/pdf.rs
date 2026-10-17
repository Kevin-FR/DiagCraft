//! Portable vector PDF export. Fonts are bundled; no browser or printer is required.
use crate::{document::Document, svg};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageFormat {
    #[default]
    Drawing,
    A4Landscape,
    A4Portrait,
    LetterLandscape,
}
impl PageFormat {
    pub const ALL: [Self; 4] = [
        Self::Drawing,
        Self::A4Landscape,
        Self::A4Portrait,
        Self::LetterLandscape,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Drawing => "Tailles propres aux pages",
            Self::A4Landscape => "A4 paysage",
            Self::A4Portrait => "A4 portrait",
            Self::LetterLandscape => "Lettre US paysage",
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "dessin" => Ok(Self::Drawing),
            "a4-paysage" => Ok(Self::A4Landscape),
            "a4-portrait" => Ok(Self::A4Portrait),
            "lettre" => Ok(Self::LetterLandscape),
            _ => Err("Format PDF inconnu.".into()),
        }
    }
    pub fn size_px(self, doc: &Document) -> [f32; 2] {
        match self {
            Self::Drawing => doc.page_size,
            Self::A4Landscape => [1122.5197, 793.7008],
            Self::A4Portrait => [793.7008, 1122.5197],
            Self::LetterLandscape => [1056., 816.],
        }
    }
}
fn tree(doc: &Document, format: PageFormat) -> Result<(svg2pdf::usvg::Tree, [f32; 2]), String> {
    let source = svg::export(doc)?;
    let size = format.size_px(doc);
    let margin = if format == PageFormat::Drawing {
        0.
    } else {
        32.
    };
    let scale = ((size[0] - 2. * margin) / doc.page_size[0])
        .min((size[1] - 2. * margin) / doc.page_size[1]);
    let [w, h] = doc.page_size.map(|v| v * scale);
    let root_end = source.find("role=\"img\">\n").ok_or("SVG incomplet")? + "role=\"img\">\n".len();
    let inner = &source[root_end..source.len() - "</svg>\n".len()];
    let wrapped = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><svg x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {} {}\">{inner}</svg></svg>",
        size[0],
        size[1],
        size[0],
        size[1],
        (size[0] - w) / 2.,
        (size[1] - h) / 2.,
        doc.page_size[0],
        doc.page_size[1]
    );
    let mut options = svg2pdf::usvg::Options::default();
    for bytes in [
        include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/SourceSans3-Bold.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/SourceSans3-It.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/SourceSans3-BoldIt.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/Hack-Regular.ttf").as_slice(),
    ] {
        options.fontdb_mut().load_font_data(bytes.to_vec());
    }
    options.fontdb_mut().set_sans_serif_family("Source Sans 3");
    options.fontdb_mut().set_serif_family("Source Sans 3");
    options.fontdb_mut().set_monospace_family("Hack");
    let tree = svg2pdf::usvg::Tree::from_str(&wrapped, &options).map_err(|e| e.to_string())?;
    Ok((tree, size))
}
pub fn export_current(doc: &Document, format: PageFormat) -> Result<Vec<u8>, String> {
    let (tree, size) = tree(doc, format)?;
    let dpi = 96_f32.max(size[0].max(size[1]) * 72. / 14400.);
    svg2pdf::to_pdf(
        &tree,
        conversion_options(&tree),
        svg2pdf::PageOptions { dpi },
    )
    .map_err(|e| e.to_string())
}
fn conversion_options(tree: &svg2pdf::usvg::Tree) -> svg2pdf::ConversionOptions {
    fn visit(g: &svg2pdf::usvg::Group, scale: &mut f32) {
        if !g.filters().is_empty()
            && let Some(b) = g.layer_bounding_box().transform(g.transform())
        {
            *scale = scale.min(2048. / b.width().max(b.height()));
        }
        for n in g.children() {
            if let svg2pdf::usvg::Node::Group(child) = n {
                visit(child, scale);
            }
            n.subroots(|child| visit(child, scale));
        }
    }
    // Only filtered shadows are rasterized; text, fills and contours stay vector.
    // Bound each temporary filter image even for very large custom pages.
    let mut scale = 1.5;
    visit(tree.root(), &mut scale);
    svg2pdf::ConversionOptions {
        raster_scale: scale,
        ..Default::default()
    }
}
pub fn export(doc: &Document, format: PageFormat) -> Result<Vec<u8>, String> {
    doc.validate()?;
    if doc.page_count() == 1 {
        return export_current(doc, format);
    }
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, TextStr};
    let mut pdf = Pdf::new();
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let pages = alloc.bump();
    let mut kids = vec![];
    pdf.catalog(catalog).pages(pages);
    for i in 0..doc.page_count() {
        let d = doc.page_document(i).unwrap();
        let (tree, size) = tree(&d, format)?;
        let (chunk, xobject) =
            svg2pdf::to_chunk(&tree, conversion_options(&tree)).map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let xobject = map[&xobject];
        pdf.extend(&chunk);
        let page_id = alloc.bump();
        let content_id = alloc.bump();
        kids.push(page_id);
        let dpi = 96_f32.max(size[0].max(size[1]) * 72. / 14400.);
        let [w, h] = size.map(|v| v * 72. / dpi);
        let mut page = pdf.page(page_id);
        page.parent(pages)
            .media_box(Rect::new(0., 0., w, h))
            .contents(content_id);
        page.resources().x_objects().pair(Name(b"Drawing"), xobject);
        page.finish();
        let mut content = Content::new();
        content
            .transform([w, 0., 0., h, 0., 0.])
            .x_object(Name(b"Drawing"));
        pdf.stream(content_id, &content.finish());
    }
    pdf.pages(pages)
        .kids(kids.iter().copied())
        .count(kids.len() as i32);
    pdf.document_info(alloc.bump())
        .title(TextStr(&doc.title))
        .producer(TextStr("DiagCraft"));
    Ok(pdf.finish())
}
