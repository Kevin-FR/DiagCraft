use super::*;
use diagcraft::agent;

pub(super) struct AgentPanel {
    context: String,
    patch: String,
    preview: Option<agent::Preview>,
    error: Option<String>,
    tab: u8,
    page: usize,
}
impl DrawApp {
    pub(super) fn open_agent_panel(&mut self) {
        self.commit();
        match context_for_agent(&self.doc) {
            Ok(context) => {
                self.agent_panel = Some(AgentPanel {
                    context,
                    patch: String::new(),
                    preview: None,
                    error: None,
                    tab: 0,
                    page: self.doc.page_index,
                })
            }
            Err(e) => self.error = Some(e),
        }
        self.context_open = false;
    }
    pub(super) fn apply_agent_patch(&mut self, text: &str) -> Result<(), String> {
        // Recompute against the current in-memory document, including unsaved edits.
        let patch = agent::parse_patch(text)?;
        let preview = agent::preview(&self.doc, &patch)?;
        self.commit();
        self.doc = preview.document;
        self.selected.clear();
        self.drag = None;
        self.set_tool(Tool::Select);
        self.commit();
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
            ui.set_width(1020.);
            ui.horizontal(|ui|{ui.heading(t("Travailler avec un agent"));ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button(t("Fermer")).clicked(){close=true;}});});
            ui.label(RichText::new(t("Claude, ChatGPT ou tout autre agent ayant accès à vos fichiers ou à MCP.")).color(MUTED));
            ui.add_space(12.);
            ui.horizontal(|ui|{for (tab,label) in [(0,"1  Partager le contexte"),(1,"2  Examiner le résultat"),(2,"CLI et connexion MCP")] {ui.selectable_value(&mut panel.tab,tab,t(label));}});
            ui.add_space(10.);ui.separator();ui.add_space(10.);
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
                    if ui.button(RichText::new(t("Vérifier et prévisualiser")).strong()).clicked(){match agent::parse_patch(&panel.patch).and_then(|p|agent::preview(&self.doc,&p)){Ok(p)=>{panel.preview=Some(p);panel.error=None;},Err(e)=>{panel.preview=None;panel.error=Some(e);}}}
                });
                ui.add_space(10.);
                ui.columns(2,|cols| {
                    cols[0].label(RichText::new(t("PATCH JSON")).strong().color(MUTED));
                    egui::ScrollArea::vertical().id_salt("agent_patch_scroll").max_height(390.).show(&mut cols[0],|ui|{if ui.add(egui::TextEdit::multiline(&mut panel.patch).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(24).char_limit(document::MAX_FILE_BYTES as usize).hint_text("{\n  \"api_version\": 1,\n  \"expected_revision\": \"sha256:…\",\n  \"operations\": […]\n}")).changed(){panel.preview=None;panel.error=None;}});
                    let ui=&mut cols[1];
                    if let Some(preview)=&panel.preview {
                        panel.page=panel.page.min(preview.document.page_count()-1);
                        ui.horizontal(|ui|{ui.label(RichText::new(t("APERÇU DU RÉSULTAT")).strong().color(ACCENT));egui::ComboBox::from_id_salt("agent_preview_page").selected_text(i18n::render("Page {0}", &[("0", format!("{}", panel.page+1))])).show_ui(ui,|ui|{for (i,name) in preview.document.page_names().iter().enumerate(){ui.selectable_value(&mut panel.page,i,*name);}});});
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),230.),Sense::hover());
                        library_ui::paint_document_preview(ui.painter(),&preview.document.page_document(panel.page).unwrap(),r);
                        ui.label(RichText::new(i18n::render("{0} opérations · {1} pages · {2} objets", &[("0", format!("{}", preview.report["changes"].as_array().unwrap().len())),("1", format!("{}", preview.document.page_count())),("2", format!("{}", preview.document.object_count()))])).strong());
                        egui::ScrollArea::vertical().id_salt("agent_report_scroll").max_height(125.).show(ui,|ui|{
                            for c in preview.report["changes"].as_array().unwrap() {
                                let object = c["id"].as_u64().map(|id| i18n::render(" · objet {0}", &[("0", id.to_string())])).unwrap_or_default();
                                ui.label(i18n::render("{0}  {1} · page {2}{3}", &[("0",(c["operation"].as_u64().unwrap()+1).to_string()),("1",c["description"].as_str().unwrap().into()),("2",c["page"].to_string()),("3",object)]));
                            }
                            for w in preview.report["diagnostics"].as_array().unwrap(){ui.colored_label(Color32::from_rgb(178,104,22),i18n::render("Hors page : page {0} · objet {1}", &[("0", format!("{}", w["page"])),("1", format!("{}", w["id"]))]));}
                        });
                    } else {
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),230.),Sense::hover());library_ui::paint_document_preview(ui.painter(),&self.doc.page_document(self.doc.page_index).unwrap(),r);
                        ui.label(RichText::new(t("Votre dessin actuel")).strong());ui.label(t("Vérifiez le patch pour afficher le résultat proposé."));
                    }
                });
                ui.add_space(12.);
                ui.horizontal(|ui|{apply=ui.add_enabled(panel.preview.is_some(),egui::Button::new(RichText::new(t("Appliquer les modifications")).strong().color(Color32::WHITE)).fill(ACCENT).stroke(Stroke::NONE)).clicked();ui.label(t("Un seul ⌘ Z annule tout le lot."));});
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
            if let Some(error)=&panel.error {ui.add_space(8.);ui.colored_label(Color32::from_rgb(190,50,50),error);}
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
        if let Some(panel) = &mut self.agent_panel {
            panel.tab = 1;
            panel.patch = patch;
            panel.preview = Some(preview);
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
