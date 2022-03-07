// #![feature(plugin)]
// #![plugin(clippy)]

extern crate cassandra_cpp_sys;

mod examples_util;
use cassandra_cpp_sys::*;
use examples_util::*;
use std::ffi::CString;

struct Pair {
    key: String,
    value: String,
}

fn prepare_insert_into_batch<'a>(session: &mut CassSession) -> Result<&'a CassPrepared, CassError> {
    unsafe {
        let query = CString::new("INSERT INTO examples.pairs (key, value) VALUES (?, ?)").unwrap();

        let future = &mut *cass_session_prepare(session, query.as_ptr());
        cass_future_wait(future);
        let rc = cass_future_error_code(future);
        let result = match rc {
            CASS_OK => {
                let prepared = &*cass_future_get_prepared(future);
                Ok(prepared)
            }
            _ => {
                print_error(future);
                Err(rc)
            }
        };
        cass_future_free(future);
        result
    }
}

fn insert_into_batch_with_prepared(
    session: &mut CassSession,
    prepared: &CassPrepared,
    pairs: Vec<Pair>,
) -> Result<(), CassError> {
    unsafe {
        let batch = cass_batch_new(CASS_BATCH_TYPE_LOGGED);

        for pair in pairs {
            let statement = cass_prepared_bind(prepared);
            let key = CString::new(pair.key).unwrap();
            let value = CString::new(pair.value).unwrap();

            cass_statement_bind_string(statement, 0, key.as_ptr());
            cass_statement_bind_string(statement, 1, value.as_ptr());
            cass_batch_add_statement(batch, statement);
            cass_statement_free(statement);
        }

        let statement1 =
            CString::new("INSERT INTO examples.pairs (key, value) VALUES ('c', '3')").unwrap();
        let statement1 = cass_statement_new(statement1.as_ptr(), 0);

        cass_batch_add_statement(batch, statement1);
        cass_statement_free(statement1);

        let statement2 =
            CString::new("INSERT INTO examples.pairs (key, value) VALUES (?, ?)").unwrap();

        let statement2 = cass_statement_new(statement2.as_ptr(), 2);

        let key = CString::new("d").unwrap();
        let value = CString::new("4").unwrap();

        cass_statement_bind_string(statement2, 0, key.as_ptr());
        cass_statement_bind_string(statement2, 1, value.as_ptr());

        cass_batch_add_statement(batch, statement2);
        cass_statement_free(statement2);

        let future = &mut *cass_session_execute_batch(session, batch);
        cass_future_wait(future);

        let rc = cass_future_error_code(future);
        let result = match rc {
            CASS_OK => {
                println!("Batch successfully inserted");
                Ok(())
            }
            _ => {
                print_error(future);
                Err(rc)
            }
        };

        cass_future_free(future);
        cass_batch_free(batch);

        result
    }
}

pub fn main() {
    unsafe {
        let cluster = create_cluster();
        let session = &mut *cass_session_new();

        let pairs = vec![
            Pair {
                key: "a".to_owned(),
                value: "1".to_owned(),
            },
            Pair {
                key: "b".to_owned(),
                value: "2".to_owned(),
            },
        ];

        connect_session(session, cluster).unwrap();

        execute_query(session,
                      "CREATE KEYSPACE IF NOT EXISTS examples WITH replication = { \'class\': \'SimpleStrategy\', \
                       \'replication_factor\': \'3\' };")
            .unwrap();

        execute_query(
            session,
            "CREATE TABLE IF NOT EXISTS examples.pairs (key text, value text, PRIMARY KEY (key));",
        )
        .unwrap();

        match prepare_insert_into_batch(session) {
            Ok(prepared) => {
                insert_into_batch_with_prepared(session, prepared, pairs).unwrap();
                cass_prepared_free(prepared);
            }
            Err(rc) => println!("err: {:?}", rc),
        }

        let close_future = cass_session_close(session);
        cass_future_wait(close_future);
        cass_future_free(close_future);

        cass_cluster_free(cluster);
        cass_session_free(session);
    }
}
