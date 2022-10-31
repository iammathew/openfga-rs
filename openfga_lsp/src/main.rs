use dashmap::DashMap;
use openfga_common::AuthorizationModel;
use openfga_dsl_parser::{parse_model, Token};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    model_map: DashMap<String, Option<AuthorizationModel>>,
    token_map: DashMap<String, Option<Vec<Token>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                document_symbol_provider: Some(OneOf::Right(DocumentSymbolOptions {
                    label: Some("OpenFGA".into()),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                })),
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
        self.on_change(&params.text_document.uri, params.text_document.text);
    }

    async fn did_change(&self, _: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File saved, reparsing model!")
            .await;
        self.on_change(&params.text_document.uri, params.text.unwrap());
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
        let model = model_ref.as_ref().unwrap();
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
                    range: Range::default(),
                    selection_range: Range::default(),
                    children: Some(
                        t.relations
                            .iter()
                            .map(|r| DocumentSymbol {
                                name: r.identifier.name.clone(),
                                detail: None,
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: None,
                                range: Range::default(),
                                selection_range: Range::default(),
                                children: None,
                            })
                            .collect(),
                    ),
                })
                .collect(),
        );
        Ok(Some(res))
    }
}

impl Backend {
    fn on_change(&self, uri: &Url, text: String) {
        let model = parse_model(&text);
        match model {
            Ok(m) => self.model_map.insert(uri.to_string(), Some(m)),
            Err(_) => self.model_map.insert(uri.to_string(), None),
        };
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        model_map: DashMap::new(),
        token_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
