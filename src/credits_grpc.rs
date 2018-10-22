// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_CREDITS_GET_ACCOUNT_STATUS: ::grpcio::Method<super::credits::AccountStatusRequest, super::credits::AccountStatus> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/Credits/GetAccountStatus",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_CREDITS_ADD_CREDITS: ::grpcio::Method<super::credits::AddCreditsCommand, super::credits::AddCreditsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/Credits/AddCredits",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

pub struct CreditsClient {
    client: ::grpcio::Client,
}

impl CreditsClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        CreditsClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn get_account_status_opt(&self, req: &super::credits::AccountStatusRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::credits::AccountStatus> {
        self.client.unary_call(&METHOD_CREDITS_GET_ACCOUNT_STATUS, req, opt)
    }

    pub fn get_account_status(&self, req: &super::credits::AccountStatusRequest) -> ::grpcio::Result<super::credits::AccountStatus> {
        self.get_account_status_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_account_status_async_opt(&self, req: &super::credits::AccountStatusRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::credits::AccountStatus>> {
        self.client.unary_call_async(&METHOD_CREDITS_GET_ACCOUNT_STATUS, req, opt)
    }

    pub fn get_account_status_async(&self, req: &super::credits::AccountStatusRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::credits::AccountStatus>> {
        self.get_account_status_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn add_credits_opt(&self, req: &super::credits::AddCreditsCommand, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::credits::AddCreditsResponse> {
        self.client.unary_call(&METHOD_CREDITS_ADD_CREDITS, req, opt)
    }

    pub fn add_credits(&self, req: &super::credits::AddCreditsCommand) -> ::grpcio::Result<super::credits::AddCreditsResponse> {
        self.add_credits_opt(req, ::grpcio::CallOption::default())
    }

    pub fn add_credits_async_opt(&self, req: &super::credits::AddCreditsCommand, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::credits::AddCreditsResponse>> {
        self.client.unary_call_async(&METHOD_CREDITS_ADD_CREDITS, req, opt)
    }

    pub fn add_credits_async(&self, req: &super::credits::AddCreditsCommand) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::credits::AddCreditsResponse>> {
        self.add_credits_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Credits {
    fn get_account_status(&mut self, ctx: ::grpcio::RpcContext, req: super::credits::AccountStatusRequest, sink: ::grpcio::UnarySink<super::credits::AccountStatus>);
    fn add_credits(&mut self, ctx: ::grpcio::RpcContext, req: super::credits::AddCreditsCommand, sink: ::grpcio::UnarySink<super::credits::AddCreditsResponse>);
}

pub fn create_credits<S: Credits + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_CREDITS_GET_ACCOUNT_STATUS, move |ctx, req, resp| {
        instance.get_account_status(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_CREDITS_ADD_CREDITS, move |ctx, req, resp| {
        instance.add_credits(ctx, req, resp)
    });
    builder.build()
}
