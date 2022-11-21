use dashmap::DashMap;
use openfga_checker::{check_model, ModelError};
use openfga_common::AuthorizationModel;
use openfga_model_dsl_parser::{parse_model, Token};
use ropey::Rope;
use std::env;
use std::ops::Range as OpsRange;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    diagnostics_map: DashMap<String, Option<Vec<ModelError>>>,
    model_map: DashMap<String, Option<AuthorizationModel>>,
    rope_map: DashMap<String, Option<Rope>>,
    token_map: DashMap<String, Option<Vec<(Token, OpsRange<usize>)>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        change: Some(TextDocumentSyncKind::FULL),
                        open_close: Some(true),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                document_symbol_provider: Some(OneOf::Right(DocumentSymbolOptions {
                    label: Some("OpenFGA".into()),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                })),
                semantic_tokens_provider: if env::var("OPENFGA_DISABLE_SEMANTIC_TOKEN")
                    .unwrap_or("false".into())
                    == "true"
                {
                    None
                } else {
                    Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: Some(false),
                            },
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::OPERATOR,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::METHOD,
                                ],
                                token_modifiers: vec![],
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ))
                },
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("type".to_string(), "New type".to_string()),
            CompletionItem::new_simple("define".to_string(), "New relation".to_string()),
        ])))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File opened!")
            .await;
        self.on_change(&params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "File changed, parsing and checking model again!",
            )
            .await;
        self.on_change(
            &params.text_document.uri,
            params.content_changes[0].text.clone(),
        )
        .await
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "File saved, parsing and checking model again!",
            )
            .await;
        self.on_change(&params.text_document.uri, params.text.unwrap())
            .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File closed!")
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let model_ref = self
            .model_map
            .get(&params.text_document.uri.to_string())
            .unwrap();
        let model = match model_ref.as_ref() {
            Some(m) => m,
            None => return Ok(None),
        };
        let res: DocumentSymbolResponse = DocumentSymbolResponse::Nested(
            model
                .types
                .iter()
                .map(|t| DocumentSymbol {
                    name: t.identifier.name.clone(),
                    detail: None,
                    kind: SymbolKind::CLASS,
                    tags: None,
                    deprecated: None,
                    range: self.span_to_range(&params.text_document.uri, t.span.clone().unwrap()),
                    selection_range: self.span_to_range(
                        &params.text_document.uri,
                        t.identifier.span.clone().unwrap(),
                    ),
                    children: Some(
                        t.relations
                            .iter()
                            .map(|r| DocumentSymbol {
                                name: r.identifier.name.clone(),
                                detail: None,
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: None,
                                range: self.span_to_range(
                                    &params.text_document.uri,
                                    r.span.clone().unwrap(),
                                ),
                                selection_range: self.span_to_range(
                                    &params.text_document.uri,
                                    r.identifier.span.clone().unwrap(),
                                ),
                                children: None,
                            })
                            .collect(),
                    ),
                })
                .collect(),
        );
        Ok(Some(res))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let tokens_ref = self
            .token_map
            .get(&params.text_document.uri.to_string())
            .unwrap();
        let tokens = match tokens_ref.as_ref() {
            Some(t) => t,
            None => return Ok(None),
        };

        let rope_ref = self
            .rope_map
            .get(&params.text_document.uri.to_string())
            .unwrap();
        let rope = rope_ref.as_ref().unwrap();

        let mut prev_line: usize = 0;
        let mut prev_char: usize = 0;

        let res = SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens
                .iter()
                .filter(|(t, _)| !(t == &Token::OpenParenthesis || t == &Token::CloseParenthesis))
                .map(|(t, r)| {
                    let line = rope.char_to_line(r.start);
                    let delta_line = line - prev_line;
                    prev_line = line;
                    let mut delta_start = r.start - rope.line_to_char(line);
                    let len = r.len();
                    if delta_line == 0 {
                        let char = delta_start;
                        delta_start = delta_start - prev_char;
                        prev_char = char;
                    } else {
                        prev_char = delta_start;
                    }
                    return SemanticToken {
                        delta_line: delta_line as u32,
                        delta_start: delta_start as u32,
                        token_type: match t {
                            Token::Type => 0,
                            Token::Define => 0,
                            Token::Relations => 0,
                            Token::As => 0,
                            Token::And => 1,
                            Token::Or => 1,
                            Token::From => 1,
                            Token::But => 1,
                            Token::Not => 1,
                            Token::OpenParenthesis => 1,
                            Token::CloseParenthesis => 1,
                            Token::Identifier(_) => 2,
                            Token::SelfRef => 3,
                        },
                        length: len as u32,
                        ..Default::default()
                    };
                })
                .collect(),
        });

        Ok(Some(res))
    }
}

impl Backend {
    async fn on_change(&self, uri: &Url, text: String) {
        let res = parse_model(&text);
        match res {
            Ok((model, tokens)) => {
                let check = check_model(&model);
                match check {
                    Ok(_) => {
                        self.diagnostics_map.insert(uri.to_string(), None);
                    }
                    Err(errors) => {
                        self.diagnostics_map.insert(uri.to_string(), Some(errors));
                    }
                };
                self.rope_map
                    .insert(uri.to_string(), Some(Rope::from_str(&text)));
                self.model_map.insert(uri.to_string(), Some(model));
                self.token_map.insert(uri.to_string(), Some(tokens));
                self.publish_diagnostics(uri).await;
            }
            Err(_) => {
                if !self.model_map.contains_key(&uri.to_string()) {
                    self.rope_map.insert(uri.to_string(), None);
                    self.model_map.insert(uri.to_string(), None);
                    self.token_map.insert(uri.to_string(), None);
                };
            }
        };
    }

    async fn publish_diagnostics(&self, uri: &Url) {
        let diagnostics = self
            .diagnostics_map
            .get(&uri.to_string())
            .unwrap()
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|e| self.map_model_error_to_diagnostic(uri, e))
            .collect();
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn map_model_error_to_diagnostic(&self, uri: &Url, error: &ModelError) -> Diagnostic {
        return Diagnostic {
            range: self.span_to_range(&uri, error.get_span()),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::Number(error.get_code() as i32)),
            code_description: None,
            source: Some(String::from("openfga")),
            message: format!("{}", error),
            related_information: None,
            tags: None,
            data: None,
        };
    }

    fn span_to_range(&self, uri: &Url, span: OpsRange<usize>) -> Range {
        Range {
            start: self.char_to_position(uri, span.start),
            end: self.char_to_position(uri, span.end),
        }
    }

    fn char_to_position(&self, uri: &Url, char: usize) -> Position {
        let rope_ref = self.rope_map.get(&uri.to_string()).unwrap();
        let rope = rope_ref.as_ref().unwrap();
        let line = rope.char_to_line(char);
        let start = rope.line_to_char(line);
        Position {
            line: line as u32,
            character: (char - start) as u32,
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        diagnostics_map: DashMap::new(),
        model_map: DashMap::new(),
        token_map: DashMap::new(),
        rope_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
