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
    Diff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiffCategory {
    Added,
    Modified,
    Removed,
}

/// Jeu de différences calculé une seule fois depuis le rapport du patch.
/// Clé `(page, id)` : les identifiants d’objets sont uniques par page.
#[derive(Default)]
pub(super) struct DiffSet {
    keys: std::collections::HashMap<(usize, u64), DiffCategory>,
}

impl DiffSet {
    fn from_report(source: &Document, result: &Document, report: &serde_json::Value) -> Self {
        let mut keys = std::collections::HashMap::new();
        if let Some(changes) = report["changes"].as_array() {
            for c in changes {
                let Some(page) = c["page"].as_u64().map(|p| p as usize) else {
                    continue;
                };
                // Un changement sans identifiant (ex. réorganisation de pages)
                // reste décrit en texte mais n’est pas surlignable.
                let Some(id) = c["id"].as_u64() else {
                    continue;
                };
                let has = |doc: &Document| {
                    doc.page_document(page)
                        .map(|d| diagcraft::agent::object_ids(&d).contains(&id))
                        .unwrap_or(false)
                };
                let category = match (has(source), has(result)) {
                    (true, true) => DiffCategory::Modified,
                    (false, true) => DiffCategory::Added,
                    (true, false) => DiffCategory::Removed,
                    (false, false) => continue,
                };
                keys.insert((page, id), category);
            }
        }
        DiffSet { keys }
    }

    /// Surlignage pour la vue Après (mode Diff) : ajouts + modifications en
    /// accent ; les suppressions n’y figurent plus.
    fn after_highlight(
        &self,
        page: usize,
    ) -> (
        std::collections::HashSet<u64>,
        std::collections::HashSet<u64>,
    ) {
        let mut accent = std::collections::HashSet::new();
        for (&(p, id), cat) in &self.keys {
            if p == page && matches!(cat, DiffCategory::Added | DiffCategory::Modified) {
                accent.insert(id);
            }
        }
        (accent, std::collections::HashSet::new())
    }

    /// Surlignage pour la vue Avant : les modifications en accent, les
    /// suppressions en couleur d’avertissement (encore présentes avant).
    fn before_highlight(
        &self,
        page: usize,
    ) -> (
        std::collections::HashSet<u64>,
        std::collections::HashSet<u64>,
    ) {
        let mut accent = std::collections::HashSet::new();
        let mut removed = std::collections::HashSet::new();
        for (&(p, id), cat) in &self.keys {
            if p == page {
                match cat {
                    DiffCategory::Modified => {
                        accent.insert(id);
                    }
                    DiffCategory::Removed => {
                        removed.insert(id);
                    }
                    DiffCategory::Added => {}
                }
            }
        }
        (accent, removed)
    }

    fn counts(&self) -> (usize, usize, usize) {
        let (mut added, mut modified, mut removed) = (0, 0, 0);
        for cat in self.keys.values() {
            match cat {
                DiffCategory::Added => added += 1,
                DiffCategory::Modified => modified += 1,
                DiffCategory::Removed => removed += 1,
            }
        }
        (added, modified, removed)
    }
}

struct Comparison {
    source: Document,
    result: agent::Preview,
    diff: DiffSet,
    view: PreviewView,
    before_page: usize,
    after_page: usize,
}
impl Comparison {
    fn new(source: &Document, result: agent::Preview) -> Self {
        let diff = DiffSet::from_report(source, &result.document, &result.report);
        let before_page = source.page_index;
        let after_page = result.document.page_index;
        Self {
            source: source.clone(),
            result,
            diff,
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
                            ui.selectable_value(&mut comparison.view,PreviewView::Diff,t("Différences"));
                        });
                        ui.small(t("Les pages sont indépendantes et peuvent être réordonnées. Chaque vue est ajustée séparément ; les différences sont surlignées."));
                        let (is_before, accent_set, removed_set, tint) = match comparison.view {
                            PreviewView::Before => {
                                let (a, r) = comparison.diff.before_highlight(comparison.before_page);
                                (true, a, r, false)
                            }
                            PreviewView::After | PreviewView::Diff => {
                                let (a, r) = comparison.diff.after_highlight(comparison.after_page);
                                (false, a, r, comparison.view == PreviewView::Diff)
                            }
                        };
                        let (document, page): (&Document, &mut usize) = if is_before {
                            (&comparison.source, &mut comparison.before_page)
                        } else {
                            (&comparison.result.document, &mut comparison.after_page)
                        };
                        let diff = library_ui::PreviewDiff {
                            accent: &accent_set,
                            removed: &removed_set,
                            tint,
                        };
                        *page=(*page).min(document.page_count().saturating_sub(1));
                        let (added,modified,removed)=comparison.diff.counts();
                        ui.horizontal(|ui|{ui.label(RichText::new(t("APERÇU")).strong().color(ACTION_TEXT));egui::ComboBox::from_id_salt(("agent_preview_page",comparison.view == PreviewView::Before)).selected_text(i18n::render("Page {0}", &[("0", format!("{}", *page+1))])).show_ui(ui,|ui|{for (i,name) in document.page_names().iter().enumerate(){ui.selectable_value(page,i,*name);}});if comparison.view == PreviewView::Diff {ui.separator();ui.label(RichText::new(t("Comparaison visuelle")).strong());ui.colored_label(ACTION_TEXT,i18n::render("{0} ajout(s)", &[("0", format!("{}", added))]));ui.colored_label(ACTION_TEXT,i18n::render("{0} modification(s)", &[("0", format!("{}", modified))]));ui.colored_label(ERROR_TEXT,i18n::render("{0} suppression(s)", &[("0", format!("{}", removed))]));}});
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),230.),Sense::hover());
                        library_ui::paint_document_preview_with(ui.painter(),&document.page_document(*page).unwrap(),r,Some(&diff));
                        if comparison.view == PreviewView::Diff {
                            ui.small(t("Objets modifiés par l’agent, surlignés d’accent dans l’après ; les suppressions sont signalées dans l’avant."));
                        }
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

    #[test]
    fn diff_set_categorizes_added_modified_and_removed() {
        let mut doc = Document::default();
        let keep = doc.add_node(ShapeKind::Rectangle, [50., 50.]);
        let remove = doc.add_node(ShapeKind::Ellipse, [150., 50.]);
        doc.node_mut(remove).unwrap().text = "À supprimer".into();
        let page = doc.page_index;
        let patch = format!(
            r#"{{"api_version":1,"expected_revision":"{}","operations":[
                {{"op":"add_node","page":{},"alias":"nouveau","kind":"rounded","position":[80,200],"size":[120,60],"text":"Ajouté"}},
                {{"op":"update_node","page":{},"target":{},"changes":{{"text":"Modifié"}}}},
                {{"op":"delete","page":{},"target":{}}}
            ]}}"#,
            agent::revision(&doc),
            page,
            page,
            keep,
            page,
            remove
        );
        let parsed = agent::parse_patch(&patch).unwrap();
        let result = agent::preview(&doc, &parsed).unwrap();
        let diff = DiffSet::from_report(&doc, &result.document, &result.report);
        let (added, modified, removed) = diff.counts();
        assert_eq!(added, 1);
        assert_eq!(modified, 1);
        assert_eq!(removed, 1);
        // Dans l’après (mode Diff), les ajouts et modifications sont surlignés en accent.
        let (accent, accent_removed) = diff.after_highlight(page);
        assert!(accent_removed.is_empty());
        assert!(accent.contains(&keep));
        let new_id = result
            .document
            .page_document(page)
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.text == "Ajouté")
            .unwrap()
            .id;
        assert!(accent.contains(&new_id));
        // Dans l’avant, l’objet supprimé est signalé comme retiré.
        let (before_accent, before_removed) = diff.before_highlight(page);
        assert!(before_accent.contains(&keep));
        assert!(before_removed.contains(&remove));
    }

    #[test]
    fn switching_to_diff_view_renders_legend_and_counts() {
        let (ctx, mut app) = app_with_pages();
        ctx.enable_accesskit();
        app.open_agent_panel();
        let patch = example_patch(&app.doc);
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
        // Bascule vers le mode Différences.
        let label = t("Différences");
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.size() / 2.)
                }
                _ => None,
            })
            .expect("Différences button must be visible");
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
            PreviewView::Diff
        );
        // La légende de la comparaison visuelle est affichée avec les compteurs.
        let frame_output = frame(&mut app, vec![]);
        let texts: Vec<&str> = frame_output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|txt| *txt == t("Comparaison visuelle")),
            "legend title missing in Diff view; got {texts:?}"
        );
        assert!(
            texts.iter().any(|txt| txt.starts_with("0 ")),
            "count labels missing in Diff view; got {texts:?}"
        );
    }
}
