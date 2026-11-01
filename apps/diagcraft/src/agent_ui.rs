use super::*;
use diagcraft::agent;

pub(super) struct AgentPanel {
    context: String,
    patch: String,
    preview: Option<Comparison>,
    error: Option<String>,
    tab: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewView {
    Before,
    After,
}

struct Comparison {
    source: Document,
    result: agent::Preview,
    view: PreviewView,
    before_page: usize,
    after_page: usize,
}
impl Comparison {
    fn new(source: &Document, result: agent::Preview) -> Self {
        let before_page = source.page_index;
        let after_page = result.document.page_index;
        Self {
            source: source.clone(),
            result,
            view: PreviewView::After,
            before_page,
            after_page,
        }
    }
}
impl DrawApp {
    pub(super) fn open_agent_panel(&mut self) {
        let was_editing = self.rich_editor.is_some();
        if !self.commit_inline_if_needed() {
            return;
        }
        if !was_editing {
            self.commit();
        }
        match context_for_agent(&self.doc) {
            Ok(context) => {
                self.agent_panel = Some(AgentPanel {
                    context,
                    patch: String::new(),
                    preview: None,
                    error: None,
                    tab: 0,
                })
            }
            Err(e) => self.error = Some(e),
        }
        self.context_open = false;
    }
    pub(super) fn apply_agent_patch(&mut self, text: &str) -> Result<(), String> {
        // Preserve the inline draft before revalidating against the current document.
        let was_editing = self.rich_editor.is_some();
        if !self.commit_inline_if_needed() {
            return Err(t("Corrigez la saisie avant de continuer."));
        }
        if !was_editing {
            self.commit();
        }
        let patch = agent::parse_patch(text)?;
        let preview = agent::preview(&self.doc, &patch)?;
        self.doc = preview.document;
        self.selected.clear();
        self.drag = None;
        self.set_tool(Tool::Select);
        // preview() has validated every operation against this exact revision,
        // including an explicit unlock followed by an edit in the same batch.
        // Record that verified result directly; a before/after lock guard would
        // incorrectly reject the already authorized intermediate unlock.
        self.normalize_selection();
        self.history.record(&self.doc);
        self.property_edit = false;
        self.fit_next = true;
        self.status = i18n::render(
            "Agent : {0} opérations appliquées · ⌘ Z pour annuler.",
            &[("0", format!("{}", patch.operations.len()))],
        );
        Ok(())
    }
    pub(super) fn agent_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut panel) = self.agent_panel.take() else {
            return;
        };
        let mut close = false;
        let mut apply = false;
        let modal=egui::Modal::new("agents_panel".into()).show(ctx,|ui| {
            ui.set_width(1020_f32.min(ctx.content_rect().width()-64.));
            ui.horizontal(|ui|{ui.heading(t("Travailler avec un agent"));ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button(t("Fermer")).clicked(){close=true;}});});
            ui.label(RichText::new(t("Claude, ChatGPT ou tout autre agent ayant accès à vos fichiers ou à MCP.")).color(INFO_TEXT));
            ui.add_space(12.);
            ui.horizontal(|ui|{for (tab,label) in [(0,"1  Partager le contexte"),(1,"2  Examiner le résultat"),(2,"CLI et connexion MCP")] {ui.selectable_value(&mut panel.tab,tab,t(label));}});
            ui.add_space(10.);ui.separator();ui.add_space(10.);
            // Keep the title, tabs and Close button reachable on short screens.
            // The body may contain JSON, reports or long diagnostics; its own
            // scroll budget must include all controls rather than only editors.
            egui::ScrollArea::vertical().id_salt(("agent_body_scroll",panel.tab))
                .max_height((ctx.content_rect().height()-200.).clamp(180.,620.))
                .auto_shrink([false,true]).show(ui,|ui| {
            if panel.tab==0 {
                ui.label(t("Le contexte décrit toutes les pages, les objets, leurs textes, positions et connexions, y compris votre travail non enregistré."));
                ui.horizontal(|ui| {
                    if ui.button(t("Copier le contexte JSON")).clicked(){ctx.copy_text(panel.context.clone());self.status="Contexte copié.".into();}
                    if ui.button(t("Exporter le contexte…")).clicked() && let Some(path)=rfd::FileDialog::new().add_filter(t("Contexte JSON"),&["json"]).set_file_name("contexte-diagcraft.json").save_file() {if let Err(e)=document::atomic_write(&path,panel.context.as_bytes()){panel.error=Some(e);}else{self.status="Contexte exporté.".into();}}
                    if ui.button(t("Actualiser")).clicked(){match context_for_agent(&self.doc){Ok(v)=>{panel.context=v;panel.preview=None;panel.error=None;},Err(e)=>panel.error=Some(e)}}
                });
                ui.add_space(12.);
                ui.horizontal_wrapped(|ui|{ui.label(RichText::new(i18n::render("{0} pages · {1} objets", &[("0", format!("{}", self.doc.page_count())),("1", format!("{}", self.doc.object_count()))])).strong());ui.separator();ui.label(t("px · origine en haut à gauche · pages à partir de 0"));});
                if !self.selected.is_empty(){ui.label(i18n::render("Sélection actuelle — page {0} : {1}", &[("0", format!("{}", self.doc.page_index)),("1", format!("{:?}", self.selected))]));}
                egui::ScrollArea::vertical().id_salt("agent_context_scroll").max_height(340.).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut panel.context).font(egui::TextStyle::Monospace).interactive(false).desired_width(f32::INFINITY));});
                ui.add_space(10.);ui.label(t("Demandez à l’agent un patch API 1 avec la révision de ce contexte. Le catalogue et le schéma sont accessibles ci-dessous."));
                if ui.button(t("Exporter le catalogue et le schéma…")).clicked() && let Some(path)=rfd::FileDialog::new().add_filter(t("Catalogue JSON"),&["json"]).set_file_name("catalogue-diagcraft.json").save_file() && let Err(e)=document::atomic_write(&path,serde_json::to_string_pretty(&agent::catalog()).unwrap().as_bytes()){panel.error=Some(e);}
            } else if panel.tab==1 {
                ui.label(t("Collez ou chargez le patch renvoyé par l’agent. La vérification prépare l’aperçu ; l’application reste annulable en une étape."));
                ui.horizontal(|ui| {
                    if ui.button(t("Charger un patch…")).clicked() && let Some(path)=rfd::FileDialog::new().add_filter(t("Patch JSON"),&["json"]).pick_file(){match agent::read_patch(&path){Ok(p)=>{panel.patch=serde_json::to_string_pretty(&p).unwrap();panel.preview=None;panel.error=None;},Err(e)=>panel.error=Some(e)}}
                    if ui.button(t("Exemple sur ce document")).clicked(){panel.patch=example_patch(&self.doc);panel.preview=None;panel.error=None;}
                    if ui.button(RichText::new(t("Vérifier et prévisualiser")).strong()).clicked(){match agent::parse_patch(&panel.patch).and_then(|p|agent::preview(&self.doc,&p)){Ok(p)=>{panel.preview=Some(Comparison::new(&self.doc,p));panel.error=None;},Err(e)=>{panel.preview=None;panel.error=Some(e);}}}
                });
                ui.add_space(10.);
                ui.columns(2,|cols| {
                    cols[0].label(RichText::new(t("PATCH JSON")).strong().color(INFO_TEXT));
                    egui::ScrollArea::vertical().id_salt("agent_patch_scroll").max_height(390.).show(&mut cols[0],|ui|{if ui.add(egui::TextEdit::multiline(&mut panel.patch).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(24).char_limit(document::MAX_FILE_BYTES as usize).hint_text("{\n  \"api_version\": 1,\n  \"expected_revision\": \"sha256:…\",\n  \"operations\": […]\n}")).changed(){panel.preview=None;panel.error=None;}});
                    let ui=&mut cols[1];
                    if let Some(comparison)=&mut panel.preview {
                        ui.horizontal_wrapped(|ui|{
                            ui.selectable_value(&mut comparison.view,PreviewView::Before,t("Avant"));
                            ui.selectable_value(&mut comparison.view,PreviewView::After,t("Après"));
                        });
                        ui.small(t("Les pages sont indépendantes et peuvent être réordonnées. Chaque vue est ajustée séparément ; les différences ne sont pas surlignées."));
                        let (document,page)=match comparison.view {
                            PreviewView::Before => (&comparison.source,&mut comparison.before_page),
                            PreviewView::After => (&comparison.result.document,&mut comparison.after_page),
                        };
                        *page=(*page).min(document.page_count()-1);
                        ui.horizontal(|ui|{ui.label(RichText::new(t("APERÇU")).strong().color(ACTION_TEXT));egui::ComboBox::from_id_salt(("agent_preview_page",comparison.view == PreviewView::Before)).selected_text(i18n::render("Page {0}", &[("0", format!("{}", *page+1))])).show_ui(ui,|ui|{for (i,name) in document.page_names().iter().enumerate(){ui.selectable_value(page,i,*name);}});});
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),230.),Sense::hover());
                        library_ui::paint_document_preview(ui.painter(),&document.page_document(*page).unwrap(),r);
                        let preview=&comparison.result;
                        ui.label(RichText::new(i18n::render("{0} opérations · {1} pages · {2} objets", &[("0", format!("{}", preview.report["changes"].as_array().unwrap().len())),("1", format!("{}", preview.document.page_count())),("2", format!("{}", preview.document.object_count()))])).strong());
                        egui::ScrollArea::vertical().id_salt("agent_report_scroll").max_height(125.).show(ui,|ui|{
                            for c in preview.report["changes"].as_array().unwrap() {
                                let object = c["id"].as_u64().map(|id| i18n::render(" · objet {0}", &[("0", id.to_string())])).unwrap_or_default();
                                ui.label(i18n::render("{0}  {1} · page {2}{3}", &[("0",(c["operation"].as_u64().unwrap()+1).to_string()),("1",c["description"].as_str().unwrap().into()),("2",c["page"].to_string()),("3",object)]));
                            }
                            for w in preview.report["diagnostics"].as_array().unwrap() {
                                ui.colored_label(WARNING_TEXT,format!("{} · {}",w["code"].as_str().unwrap_or("diagnostic"),w["message"].as_str().unwrap_or_default()));
                                ui.small(i18n::render("Page {0} · objet {1}", &[("0",w["page"].to_string()),("1",w["id"].to_string())]));
                                if let Some(hint)=w["hint"].as_str(){ui.small(hint);}
                            }
                        });
                    } else {
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),230.),Sense::hover());library_ui::paint_document_preview(ui.painter(),&self.doc.page_document(self.doc.page_index).unwrap(),r);
                        ui.label(RichText::new(t("Votre dessin actuel")).strong());ui.label(t("Vérifiez le patch pour afficher le résultat proposé."));
                    }
                });
                ui.add_space(12.);
                let preview_current=panel.preview.as_ref().is_some_and(|comparison| comparison.source == self.doc);
                if panel.preview.is_some() && !preview_current { ui.colored_label(WARNING_TEXT,t("Le document a changé. Actualisez l’aperçu avant de continuer.")); }
                ui.horizontal(|ui|{apply=ui.add_enabled(preview_current,egui::Button::new(RichText::new(t("Appliquer les modifications")).strong().color(Color32::WHITE)).fill(ACTION_FILL).stroke(Stroke::NONE)).clicked();ui.label(t("Un seul ⌘ Z annule tout le lot."));});
            } else {
                ui.heading(t("Vos outils, votre modèle"));
                ui.label(t("Un agent peut utiliser le binaire de l’app sans ouvrir de fenêtre. L’accès MCP est limité au dossier que vous choisissez dans la configuration de votre client."));
                ui.add_space(12.);
                ui.monospace("diagcraft --agent inspect dessin.draw\ndiagcraft --agent apply dessin.draw patch.json resultat.draw --dry-run\ndiagcraft --mcp /chemin/vers/Diagrammes");
                ui.add_space(12.);
                ui.label(t("Le serveur propose draw_catalog, draw_inspect, draw_validate et draw_apply. Il crée un nouveau fichier pour préserver l’original. Pour le document ouvert, utilisez l’import de patch dans ce panneau."));
                ui.horizontal(|ui|{if ui.button(t("Copier le guide complet")).clicked(){ctx.copy_text(include_str!("../../../docs/AGENTS-DRAW.md").into());}
 if ui.button(t("Choisir le dossier et copier MCP…")).clicked() && let Some(root)=rfd::FileDialog::new().pick_folder(){let exe=std::env::current_exe().ok().map(|p|p.with_file_name(if cfg!(target_os="windows"){"diagcraft-cli.exe"}else{"diagcraft-cli"}).display().to_string()).unwrap_or("diagcraft-cli".into());ctx.copy_text(serde_json::to_string_pretty(&serde_json::json!({"mcpServers":{"diagcraft":{"command":exe,"args":["--mcp",root.display().to_string()]}}})).unwrap());self.status="Configuration MCP copiée avec votre dossier.".into();}});
                ui.add_space(12.);
                egui::ScrollArea::vertical().id_salt("agent_guide_scroll").max_height(290.).show(ui,|ui|{ui.label(include_str!("../../../docs/AGENTS-DRAW.md"));});
            }
            if let Some(error)=&panel.error {ui.add_space(8.);ui.colored_label(ERROR_TEXT,error);}
            });
        });
        close |= modal.should_close();
        if apply {
            match self.apply_agent_patch(&panel.patch) {
                Ok(()) => close = true,
                Err(e) => {
                    panel.error = Some(e);
                    panel.preview = None;
                }
            }
        }
        if !close {
            self.agent_panel = Some(panel);
        }
    }
    pub(super) fn capture_agents(&mut self) {
        self.open_agent_panel();
        let patch = example_patch(&self.doc);
        let preview = agent::preview(&self.doc, &agent::parse_patch(&patch).unwrap()).unwrap();
        let comparison = Comparison::new(&self.doc, preview);
        if let Some(panel) = &mut self.agent_panel {
            panel.tab = 1;
            panel.patch = patch;
            panel.preview = Some(comparison);
        }
    }
}
fn example_patch(doc: &Document) -> String {
    let operations = if let Some(n) = doc.nodes.iter().find(|n| n.kind != ShapeKind::Text) {
        serde_json::json!([{ "op":"update_node","page":doc.page_index,"target":n.id,"changes":{"text":"Demande précisée","fill":[237,241,254]} },{"op":"add_connector","page":doc.page_index,"alias":"suite_libre","start":{"node":n.id,"port":"bottom"},"end":{"point":[n.center()[0]+100.,n.position[1]+n.size[1]+170.]},"style":{"label":"À compléter","pattern":"dashed"}}])
    } else {
        serde_json::json!([{"op":"add_node","page":doc.page_index,"alias":"idee","kind":"rounded","position":[80,100],"size":[220,84],"text":"Nouvelle idée"}])
    };
    serde_json::to_string_pretty(&serde_json::json!({"api_version":1,"expected_revision":agent::revision(doc),"operations":operations})).unwrap()
}

pub(super) fn context_for_agent(doc: &Document) -> Result<String, String> {
    let mut context = agent::inspect(doc)?;
    context["agent_contract"] = serde_json::json!({"patch_schema":agent::schema(),"guide":include_str!("../../../docs/AGENTS-DRAW.md")});
    serde_json::to_string_pretty(&context).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_pages() -> (egui::Context, DrawApp) {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        doc.add_node(ShapeKind::Rectangle, [200., 200.]);
        doc.add_page(false).unwrap();
        doc.add_node(ShapeKind::Diamond, [300., 300.]);
        doc.switch_page(0).unwrap();
        let app = DrawApp::from_context(&ctx, doc, None, None);
        (ctx, app)
    }

    fn reorder_patch(doc: &Document) -> String {
        serde_json::json!({"api_version":1,"expected_revision":agent::revision(doc),"operations":[{"op":"move_page","from":0,"to":1}]}).to_string()
    }

    #[test]
    fn comparison_preserves_snapshot_and_independent_page_choices_after_reorder() {
        let (_, app) = app_with_pages();
        let original = app.doc.clone();
        let patch = agent::parse_patch(&reorder_patch(&original)).unwrap();
        let result = agent::preview(&original, &patch).unwrap();
        let mut comparison = Comparison::new(&original, result);
        assert_eq!(comparison.source, original);
        assert_eq!(
            comparison.source.page(0).unwrap().nodes[0].kind,
            ShapeKind::Rectangle
        );
        assert_eq!(
            comparison.result.document.page(0).unwrap().nodes[0].kind,
            ShapeKind::Diamond
        );
        comparison.before_page = 1;
        comparison.after_page = 0;
        comparison.view = PreviewView::Before;
        assert_eq!(comparison.before_page, 1);
        comparison.view = PreviewView::After;
        assert_eq!(comparison.after_page, 0);
        assert_eq!(app.doc, original);
    }

    #[test]
    fn opening_agent_preserves_valid_inline_edit_and_rejects_invalid_edit() {
        let (_, mut app) = app_with_pages();
        let id = app.doc.nodes[0].id;
        let original = app.doc.nodes[0].text.clone();
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.text = "Pending change".into();
        app.open_agent_panel();
        assert!(app.agent_panel.is_some());
        assert!(app.rich_editor.is_none());
        assert_eq!(app.doc.nodes[0].text, "Pending change");
        assert!(
            app.agent_panel
                .as_ref()
                .unwrap()
                .context
                .contains("Pending change")
        );
        app.undo();
        assert_eq!(app.doc.nodes[0].text, original);
        assert!(!app.history.can_undo());

        let (_, mut app) = app_with_pages();
        let original = app.doc.clone();
        let id = app.doc.nodes[0].id;
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.font_size = 201.;
        app.open_agent_panel();
        assert!(app.agent_panel.is_none());
        assert!(app.rich_editor.is_some());
        assert_eq!(app.doc, original);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn applying_agent_patch_revalidates_valid_draft_and_keeps_invalid_draft() {
        let (_, mut app) = app_with_pages();
        let patch = reorder_patch(&app.doc);
        let id = app.doc.nodes[0].id;
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.text = "Newer text".into();
        assert!(app.apply_agent_patch(&patch).is_err());
        assert!(app.rich_editor.is_none());
        assert_eq!(app.doc.nodes[0].text, "Newer text");
        assert_eq!(app.doc.page(0).unwrap().nodes[0].kind, ShapeKind::Rectangle);
        app.undo();
        assert_ne!(app.doc.nodes[0].text, "Newer text");
        assert!(!app.history.can_undo());

        let (_, mut app) = app_with_pages();
        let patch = reorder_patch(&app.doc);
        let before = app.doc.clone();
        let id = app.doc.nodes[0].id;
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.font_size = 201.;
        assert!(app.apply_agent_patch(&patch).is_err());
        assert!(app.rich_editor.is_some());
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn unchanged_inline_draft_allows_patch_with_one_undo_step() {
        let (_, mut app) = app_with_pages();
        let original = app.doc.clone();
        let patch = reorder_patch(&original);
        let id = app.doc.nodes[0].id;
        app.open_node_text(id);
        app.apply_agent_patch(&patch).unwrap();
        assert!(app.rich_editor.is_none());
        assert_eq!(app.doc.page(0).unwrap().nodes[0].kind, ShapeKind::Diamond);
        app.undo();
        assert_eq!(app.doc, original);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn comparison_controls_are_accessible_and_switch_views_in_seven_languages() {
        for language in Language::ALL {
            let (ctx, mut app) = app_with_pages();
            ctx.enable_accesskit();
            app.set_language(Some(language));
            app.open_agent_panel();
            let patch = reorder_patch(&app.doc);
            let result = agent::preview(&app.doc, &agent::parse_patch(&patch).unwrap()).unwrap();
            let comparison = Comparison::new(&app.doc, result);
            let panel = app.agent_panel.as_mut().unwrap();
            panel.tab = 1;
            panel.patch = patch;
            panel.preview = Some(comparison);
            let frame = |app: &mut DrawApp, events| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1500., 1000.))),
                        events,
                        focused: true,
                        ..Default::default()
                    },
                    |ctx| app.agent_dialog(ctx),
                )
            };
            frame(&mut app, vec![]);
            let output = frame(&mut app, vec![]);
            let tree = output.platform_output.accesskit_update.unwrap();
            for label in [t("Avant"), t("Après")] {
                assert!(
                    tree.nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(label.as_str())),
                    "missing {label} in {language:?}"
                );
            }
            let before = t("Avant");
            let point = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == before => {
                        Some(text.pos + text.galley.size() / 2.)
                    }
                    _ => None,
                })
                .expect("Before button must be visible");
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(
                app.agent_panel
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap()
                    .view,
                PreviewView::Before
            );
        }
        i18n::activate(Language::French);
    }
}
