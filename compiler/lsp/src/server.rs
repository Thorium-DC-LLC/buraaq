use std::path::PathBuf;

use buraaq_fmt::{format_source, FormatOptions};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::convert::{
    file_uri, fixes_to_code_actions, hover_markdown, lsp_range, position_to_byte,
    semantic_tokens_from_source, to_lsp_types_diagnostic,
};
use crate::document::DocumentStore;
use crate::symbols::{
    call_hierarchy_item, completion_items, definition_of, hover_for_name, ident_at_offset,
    incoming_calls, keywords, outgoing_calls, references_of, signature_for_call, workspace_symbols,
};

pub struct BuraaqServer {
    client: Client,
    docs: DocumentStore,
}

impl BuraaqServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: DocumentStore::new(),
        }
    }

    async fn publish(&self, uri: &Url) {
        let Some(record) = self.docs.get(uri).await else {
            return;
        };
        let Some(analysis) = &record.analysis else {
            return;
        };
        let diags: Vec<Diagnostic> = analysis
            .diagnostics
            .iter()
            .map(|d| to_lsp_types_diagnostic(&analysis.source, d))
            .collect();
        self.client
            .publish_diagnostics(uri.clone(), diags, Some(record.version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BuraaqServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "buraaq-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::VARIABLE,
                                SemanticTokenType::NUMBER,
                                SemanticTokenType::COMMENT,
                                SemanticTokenType::OPERATOR,
                            ],
                            token_modifiers: vec![],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        ..Default::default()
                    }),
                ),
                call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Buraaq language server ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let path = uri.to_file_path().unwrap_or_else(|_| PathBuf::from("buffer.bq"));
        self.docs
            .open(
                uri.clone(),
                path,
                params.text_document.version,
                params.text_document.text,
            )
            .await;
        self.publish(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let text = params
            .content_changes
            .into_iter()
            .last()
            .map(|c| c.text)
            .unwrap_or_default();
        if self.docs.change(&uri, version, text).await.is_some() {
            self.publish(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.close(&params.text_document.uri).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        let Some((name, _)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        let Some((title, detail)) = hover_for_name(&name, &analysis.defs) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(hover_markdown(
                &title,
                &detail,
                analysis
                    .diagnostics
                    .iter()
                    .find(|d| d.message.contains(&name))
                    .and_then(|d| d.reason.as_deref()),
            )),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        let Some((name, _)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        let Some(span) = definition_of(&name, &analysis.ast, &analysis.defs) else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_range(&analysis.source, span.start.0, span.end.0),
        })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        let Some((name, _)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        let locs = references_of(&name, &analysis.ast)
            .into_iter()
            .map(|span| Location {
                uri: uri.clone(),
                range: lsp_range(&analysis.source, span.start.0, span.end.0),
            })
            .collect();
        Ok(Some(locs))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &params.position);
        let Some((_, span)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::Range(lsp_range(
            &analysis.source,
            span.start.0,
            span.end.0,
        ))))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        let Some((name, _)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        if definition_of(&name, &analysis.ast, &analysis.defs).is_none() {
            return Ok(None);
        }
        let edits: Vec<TextEdit> = references_of(&name, &analysis.ast)
            .into_iter()
            .map(|span| TextEdit {
                range: lsp_range(&analysis.source, span.start.0, span.end.0),
                new_text: new_name.clone(),
            })
            .collect();
        Ok(Some(WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(uri, edits)])),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let formatted = format_source(&record.text, &FormatOptions::official());
        if formatted == record.text.as_ref() {
            return Ok(None);
        }
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: u32::MAX,
                    character: 0,
                },
            },
            new_text: formatted,
        }]))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        for diag in &analysis.diagnostics {
            for action in fixes_to_code_actions(&analysis.source, diag) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let mut items = Vec::new();
        for kw in keywords() {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        for name in completion_items(&analysis.defs) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Buraaq definition".into()),
                ..Default::default()
            });
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        let Some((name, _)) = ident_at_offset(&analysis.ast, offset) else {
            return Ok(None);
        };
        let Some((callee, params_list)) = signature_for_call(&analysis.ast, &name) else {
            return Ok(None);
        };
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: format!("fn {callee}({})", params_list.join(", ")),
                documentation: None,
                active_parameter: None,
                parameters: Some(
                    params_list
                        .into_iter()
                        .map(|p| ParameterInformation {
                            label: ParameterLabel::Simple(p),
                            documentation: None,
                        })
                        .collect(),
                ),
            }],
            active_signature: Some(0),
            active_parameter: None,
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let fallback = buraaq_source::SourceFile::new(record.path.clone(), record.text.clone());
        let file = record
            .analysis
            .as_ref()
            .map(|a| &a.source)
            .unwrap_or(&fallback);
        Ok(Some(semantic_tokens_from_source(file)))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(
            crate::symbols::document_symbols(&analysis.source, &analysis.ast),
        )))
    }

    async fn symbol(
        &self,
        _: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let mut all = Vec::new();
        for record in self.docs.all().await {
            if let Some(analysis) = &record.analysis {
                all.extend(workspace_symbols(
                    &analysis.source,
                    &analysis.ast,
                    record.uri.clone(),
                ));
            }
        }
        Ok(Some(all))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(record) = self.docs.get(&uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        let offset = position_to_byte(&analysis.source, &pos);
        Ok(call_hierarchy_item(&analysis.source, &analysis.ast, offset).map(|i| vec![i]))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let item = params.item;
        let Some(record) = self.docs.get(&item.uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        Ok(Some(incoming_calls(
            &analysis.source,
            &analysis.ast,
            &item,
        )))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let Some(record) = self.docs.get(&item.uri).await else {
            return Ok(None);
        };
        let Some(analysis) = &record.analysis else {
            return Ok(None);
        };
        Ok(Some(outgoing_calls(
            &analysis.source,
            &analysis.ast,
            &item,
        )))
    }
}

pub async fn run_server() {
    let (service, socket) = LspService::new(|client| BuraaqServer::new(client));
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket).serve(service).await;
}
