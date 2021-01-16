use super::LanguageServerState;
use crate::{conversion::convert_uri, state::RequestHandler};
use anyhow::Result;
use dispatcher::{NotificationDispatcher, RequestDispatcher};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
};
use std::time::Instant;

pub mod dispatcher;

impl LanguageServerState {
    /// Called when a `DidOpenTextDocument` notification was received.
    fn on_did_open_text_document(
        &mut self,
        params: lsp_types::DidOpenTextDocumentParams,
    ) -> anyhow::Result<()> {
        if let Ok(path) = convert_uri(&params.text_document.uri) {
            self.open_docs.insert(path.clone());
            self.vfs
                .write()
                .set_file_contents(&path, Some(params.text_document.text.into_bytes()));
        }
        Ok(())
    }

    /// Called when a `DidChangeTextDocument` notification was received.
    fn on_did_change_text_document(
        &mut self,
        params: lsp_types::DidChangeTextDocumentParams,
    ) -> anyhow::Result<()> {
        let lsp_types::DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        if let Ok(path) = convert_uri(&text_document.uri) {
            let new_content = content_changes.get(0).unwrap().text.clone();
            self.vfs
                .write()
                .set_file_contents(&path, Some(new_content.into_bytes()));
        }
        Ok(())
    }

    /// Called when a `DidCloseTextDocument` notification was received.
    fn on_did_close_text_document(
        &mut self,
        params: lsp_types::DidCloseTextDocumentParams,
    ) -> anyhow::Result<()> {
        if let Ok(path) = convert_uri(&params.text_document.uri) {
            self.open_docs.remove(&path);
            self.vfs_monitor.reload(&path);
        }
        Ok(())
    }

    /// Called when a `DidChangeWatchedFiles` was received
    fn on_did_change_watched_files(
        &mut self,
        params: lsp_types::DidChangeWatchedFilesParams,
    ) -> anyhow::Result<()> {
        for change in params.changes {
            if let Ok(path) = convert_uri(&change.uri) {
                self.vfs_monitor.reload(&path);
            }
        }
        Ok(())
    }

    /// Handles a language server protocol request
    pub(super) fn on_request(
        &mut self,
        request: lsp_server::Request,
        request_received: Instant,
    ) -> Result<()> {
        self.register_request(&request, request_received);

        // If a shutdown was requested earlier, immediately respond with an error
        if self.shutdown_requested {
            self.respond(lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::InvalidRequest as i32,
                "shutdown was requested".to_owned(),
            ));
            return Ok(());
        }

        // Dispatch the event based on the type of event
        RequestDispatcher::new(self, request)
            .on::<lsp_types::request::Shutdown>(|state, _request| {
                state.shutdown_requested = true;
                Ok(())
            })?
            .finish();

        Ok(())
    }

    /// Handles a notification from the language server client
    pub(super) fn on_notification(&mut self, notification: lsp_server::Notification) -> Result<()> {
        NotificationDispatcher::new(self, notification)
            .on::<DidOpenTextDocument>(LanguageServerState::on_did_open_text_document)?
            .on::<DidChangeTextDocument>(LanguageServerState::on_did_change_text_document)?
            .on::<DidCloseTextDocument>(LanguageServerState::on_did_close_text_document)?
            .on::<DidChangeWatchedFiles>(LanguageServerState::on_did_change_watched_files)?
            .finish();
        Ok(())
    }

    /// Registers a request with the server. We register all these request to make sure they all get
    /// handled and so we can measure the time it takes for them to complete from the point of view
    /// of the client.
    fn register_request(&mut self, request: &lsp_server::Request, request_received: Instant) {
        self.request_queue.incoming.register(
            request.id.clone(),
            (request.method.clone(), request_received),
        )
    }

    /// Sends a request to the client and registers the request so that we can handle the response.
    pub(crate) fn send_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        handler: RequestHandler,
    ) {
        let request = self
            .request_queue
            .outgoing
            .register(R::METHOD.to_string(), params, handler);
        self.send(request.into());
    }

    /// Sends a notification to the client
    pub(crate) fn send_notification<N: lsp_types::notification::Notification>(
        &mut self,
        params: N::Params,
    ) {
        let not = lsp_server::Notification::new(N::METHOD.to_string(), params);
        self.send(not.into());
    }

    /// Handles a response to a request we made. The response gets forwarded to where we made the
    /// request from.
    pub(super) fn complete_request(&mut self, response: lsp_server::Response) {
        let handler = self.request_queue.outgoing.complete(response.id.clone());
        handler(self, response)
    }

    /// Sends a response to the client. This method logs the time it took us to reply
    /// to a request from the client.
    fn respond(&mut self, response: lsp_server::Response) {
        if let Some((_method, start)) = self.request_queue.incoming.complete(response.id.clone()) {
            let duration = start.elapsed();
            log::info!("handled req#{} in {:?}", response.id, duration);
            self.send(response.into());
        }
    }

    /// Sends a message to the client
    pub(crate) fn send(&mut self, message: lsp_server::Message) {
        self.sender
            .send(message)
            .expect("error sending lsp message to the outgoing channel")
    }
}
