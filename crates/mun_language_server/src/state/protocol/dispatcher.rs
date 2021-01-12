use super::LanguageServerState;
use crate::cancelation::is_canceled;
use crate::from_json;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A helper struct to ergonomically dispatch LSP requests to functions.
pub(crate) struct RequestDispatcher<'a> {
    state: &'a mut LanguageServerState,
    request: Option<lsp_server::Request>,
}

impl<'a> RequestDispatcher<'a> {
    /// Constructs a new dispatcher for the specified request
    pub fn new(state: &'a mut LanguageServerState, request: lsp_server::Request) -> Self {
        RequestDispatcher {
            state,
            request: Some(request),
        }
    }

    /// Try to dispatch the event as the given Request type.
    pub fn on<R>(
        &mut self,
        f: fn(&mut LanguageServerState, R::Params) -> Result<R::Result>,
    ) -> Result<&mut Self>
    where
        R: lsp_types::request::Request + 'static,
        R::Params: DeserializeOwned + 'static,
        R::Result: Serialize + 'static,
    {
        let (id, params) = match self.parse::<R>() {
            Some(it) => it,
            None => return Ok(self),
        };

        let result = f(self.state, params);
        let response = result_to_response::<R>(id, result);
        self.state.respond(response);
        Ok(self)
    }

    /// Tries to parse the request as the specified type. If the request is of the specified type,
    /// the request is transferred and any subsequent call to this method will return None. If an
    /// error is encountered during parsing of the request parameters an error is send to the
    /// client.
    fn parse<R>(&mut self) -> Option<(lsp_server::RequestId, R::Params)>
    where
        R: lsp_types::request::Request + 'static,
        R::Params: DeserializeOwned + 'static,
    {
        let req = match &self.request {
            Some(req) if req.method == R::METHOD => self.request.take().unwrap(),
            _ => return None,
        };

        match from_json(R::METHOD, req.params) {
            Ok(params) => Some((req.id, params)),
            Err(err) => {
                let response = lsp_server::Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    err.to_string(),
                );
                self.state.respond(response);
                None
            }
        }
    }

    /// Wraps-up the dispatcher. If the request was not handled, report back that this is an
    /// unknown request.
    pub fn finish(&mut self) {
        if let Some(req) = self.request.take() {
            log::error!("unknown request: {:?}", req);
            let response = lsp_server::Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                "unknown request".to_string(),
            );
            self.state.respond(response);
        }
    }
}

pub(crate) struct NotificationDispatcher<'a> {
    state: &'a mut LanguageServerState,
    notification: Option<lsp_server::Notification>,
}

impl<'a> NotificationDispatcher<'a> {
    /// Constructs a new dispatcher for the specified request
    pub fn new(state: &'a mut LanguageServerState, notification: lsp_server::Notification) -> Self {
        NotificationDispatcher {
            state,
            notification: Some(notification),
        }
    }

    /// Try to dispatch the event as the given Notification type.
    pub fn on<N>(
        &mut self,
        f: fn(&mut LanguageServerState, N::Params) -> Result<()>,
    ) -> Result<&mut Self>
    where
        N: lsp_types::notification::Notification + 'static,
        N::Params: DeserializeOwned + Send + 'static,
    {
        let notification = match self.notification.take() {
            Some(it) => it,
            None => return Ok(self),
        };
        let params = match notification.extract::<N::Params>(N::METHOD) {
            Ok(it) => it,
            Err(notification) => {
                self.notification = Some(notification);
                return Ok(self);
            }
        };
        f(self.state, params)?;
        Ok(self)
    }

    /// Wraps-up the dispatcher. If the notification was not handled, log an error.
    pub fn finish(&mut self) {
        if let Some(notification) = &self.notification {
            if !notification.method.starts_with("$/") {
                log::error!("unhandled notification: {:?}", notification);
            }
        }
    }
}

/// Converts the specified results of an LSP request into an LSP response handling any errors that
/// may have occurred.
fn result_to_response<R>(
    id: lsp_server::RequestId,
    result: Result<R::Result>,
) -> lsp_server::Response
where
    R: lsp_types::request::Request + 'static,
    R::Params: DeserializeOwned + 'static,
    R::Result: Serialize + 'static,
{
    match result {
        Ok(resp) => lsp_server::Response::new_ok(id, &resp),
        Err(e) => {
            if is_canceled(&*e) {
                lsp_server::Response::new_err(
                    id,
                    lsp_server::ErrorCode::ContentModified as i32,
                    "content modified".to_string(),
                )
            } else {
                lsp_server::Response::new_err(
                    id,
                    lsp_server::ErrorCode::InternalError as i32,
                    e.to_string(),
                )
            }
        }
    }
}
