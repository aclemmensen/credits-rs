use grpcio::{Environment, ServerBuilder, RpcContext, UnarySink};
use futures::Future;
use std::sync::Arc;
use std::io;
use std::io::Read;
use stopwatch::Stopwatch;

use eventstore::*;
use credits::{AccountStatusRequest, AccountStatus, AddCreditsCommand, AddCreditsResponse};
use credits_grpc::{Credits, create_credits};
use MyPool;
use {CreditCommand, CreditError};

#[derive(Clone)]
struct CreditsSvc {
    pool: MyPool
}

impl CreditsSvc {
    // helper that takes care of some of the boilerplate stuff around
    // handling grpc requests and responses
    fn doit<Treq, Tresp, F>(&mut self, ctx: RpcContext, req: Treq, sink: UnarySink<Tresp>, mut act: F) where
        Treq : ::protobuf::Message,
        Tresp : ::protobuf::Message,
        F : FnMut(&mut Self, Treq) -> Result<Tresp, CreditError>
    {
        let sw = Stopwatch::start_new();
        debug!("{:?}", req);
        let res = act(self, req);
        match res {
            Ok(resp) => {
                let f = sink.success(resp).map_err(|e| error!("{}", e));
                debug!("done in {}", sw.elapsed_ms());
                ctx.spawn(f)
            },
            Err(e) => error!("{:?}", e)
        }
    }
}

impl Credits for CreditsSvc {
    fn get_account_status(&mut self, ctx: RpcContext, req: AccountStatusRequest, sink: UnarySink<AccountStatus>) {
        self.doit(ctx, req, sink, |s, req| {
            let agg = load(req.account, &s.pool)?;
            let mut r = AccountStatus::new();
            r.set_amount(agg.amount);
            Ok(r)
        })
    }

    fn add_credits(&mut self, ctx: RpcContext, req: AddCreditsCommand, sink: UnarySink<AddCreditsResponse>) {
        self.doit(ctx, req, sink, |s, req| {
            let mut agg = load(req.account, &s.pool)?;
            run_and_store(&mut agg, CreditCommand::AddCredits(req.amount), &s.pool)?;
            let mut resp = AddCreditsResponse::new();
            resp.set_new_amount(agg.amount);
            Ok(resp)
        })
    }
}

pub fn start_server(pool: MyPool) {
    let env = Arc::new(Environment::new(1));
    let implementation = CreditsSvc {
        pool: pool
    };

    let service = create_credits(implementation);
    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("127.0.0.1", 5951)
        .build()
        .unwrap();

    info!("starting");
    server.start();
    io::stdin().read(&mut [0]).unwrap();
    info!("shutting down");
    server.shutdown().wait();
    info!("exiting");
}