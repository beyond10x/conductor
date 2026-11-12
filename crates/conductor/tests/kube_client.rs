//! The Kubernetes client conductord runs in a pod with (`conductor_cli::kube`), against the fake
//! API server of `tests/fake_kube/mod.rs`: the in-cluster config and what is missing out of a
//! cluster, the cluster's authority as the only root, the token read again on every call, each
//! kind's create, get, list and delete at its path, the API's `Status` as a typed error, a chunked
//! answer, the deadline, the access review and a pod's log tail.

mod fake_kube;
mod scratch;

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::path::Path;
use std::time::{Duration, Instant};

use conductor_cli::kube::{Config, ContainerState, Error, Kind, Pod};
use fake_kube::{FakeKube, NAMESPACE, TOKEN};
use scratch::Scratch;
use serde_json::{Value, json};

/// The namespace the instances live in, apart from the daemon's own.
const INSTANCES: &str = "conductor-instances";

/// The labels every object of the instance `alpha` carries.
const SELECTOR: &str = "app.kubernetes.io/managed-by=conductord,conductor/daemon=example-org";

fn object(name: &str, instance: &str) -> Value {
    json!({
        "apiVersion": "v1",
        "metadata": {
            "name": name,
            "labels": {
                "app.kubernetes.io/managed-by": "conductord",
                "conductor/daemon": "example-org",
                "conductor/instance": instance,
            },
        },
    })
}

fn other(name: &str) -> Value {
    json!({
        "apiVersion": "v1",
        "metadata": {"name": name, "labels": {"app.kubernetes.io/managed-by": "someone-else"}},
    })
}

fn case(name: &str) -> (Scratch, FakeKube) {
    let scratch = Scratch::new(Path::new("kube_client").join(name));
    let fake = FakeKube::start(scratch.path());
    (scratch, fake)
}

#[test]
fn the_in_cluster_config_reads_the_variables_and_the_directory() {
    let (_scratch, fake) = case("in-cluster");
    let port = fake.port().to_string();
    let vars = BTreeMap::from([
        ("KUBERNETES_SERVICE_HOST", "127.0.0.1"),
        ("KUBERNETES_SERVICE_PORT", port.as_str()),
    ]);
    let config = Config::from_env(fake.dir(), |name| vars.get(name).map(|v| (*v).to_owned()))
        .expect("the config in a cluster");
    assert_eq!(config.host, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(config.port, fake.port());
    assert_eq!(config.dir, fake.dir());
    let client = conductor_cli::kube::Client::new(config).expect("a client");
    assert_eq!(client.namespace(), NAMESPACE);
    client
        .list(INSTANCES, Kind::Pod, SELECTOR)
        .expect("a list over the config read from the cluster");
    assert_eq!(
        conductor_cli::kube::SERVICE_ACCOUNT_DIR,
        "/var/run/secrets/kubernetes.io/serviceaccount"
    );
}

#[test]
fn out_of_a_cluster_construction_names_what_is_missing() {
    let scratch = Scratch::new("kube_client/out-of-cluster");
    let dir = scratch.path().join("serviceaccount");
    let error = Config::from_env(&dir, |_| None).expect_err("no cluster here");
    let Error::NotInCluster { missing } = &error else {
        panic!("not a typed not-in-a-cluster error: {error:?}");
    };
    for name in ["KUBERNETES_SERVICE_HOST", "KUBERNETES_SERVICE_PORT"] {
        assert!(missing.iter().any(|m| m == name), "{name} in {missing:?}");
    }
    for file in ["ca.crt", "token", "namespace"] {
        let path = dir.join(file).display().to_string();
        assert!(missing.contains(&path), "{path} in {missing:?}");
    }
    let text = error.to_string();
    assert!(text.contains("not in a cluster"), "{text}");
    assert!(text.contains("KUBERNETES_SERVICE_HOST"), "{text}");

    // The variables alone are not a cluster either: the directory is named.
    let error = Config::from_env(&dir, |name| match name {
        "KUBERNETES_SERVICE_HOST" => Some("10.43.0.1".to_owned()),
        "KUBERNETES_SERVICE_PORT" => Some("443".to_owned()),
        _ => None,
    })
    .expect_err("no service account");
    let Error::NotInCluster { missing } = &error else {
        panic!("not a typed not-in-a-cluster error: {error:?}");
    };
    assert_eq!(missing.len(), 3, "{missing:?}");
}

#[test]
fn the_token_is_read_again_on_every_call() {
    let (_scratch, fake) = case("token");
    let client = fake.client();
    client
        .get(INSTANCES, Kind::Pod, "conductor-alpha")
        .expect_err("nothing there yet");
    fake.rotate("second-token");
    let error = client
        .get(INSTANCES, Kind::Pod, "conductor-alpha")
        .expect_err("nothing there yet");
    assert!(
        error.is_not_found(),
        "the rotated token is admitted: {error:?}"
    );
    let bearers: Vec<Option<String>> = fake.requests().into_iter().map(|r| r.bearer).collect();
    assert_eq!(
        bearers,
        [Some(TOKEN.to_owned()), Some("second-token".to_owned())]
    );
}

#[test]
fn a_wrong_token_is_a_typed_unauthorized() {
    let (_scratch, fake) = case("unauthorized");
    let client = fake.client();
    // The server admits another token than the file holds.
    fake.rotate("server-side");
    std::fs::write(fake.dir().join("token"), "stale\n").expect("write the token");
    let error = client
        .list(INSTANCES, Kind::Secret, SELECTOR)
        .expect_err("refused");
    let Error::Api(status) = &error else {
        panic!("not an API status: {error:?}");
    };
    assert_eq!(status.code, 401);
    assert_eq!(status.reason, "Unauthorized");
    assert!(!error.is_not_found() && !error.is_conflict());
}

#[test]
fn a_server_under_another_authority_is_refused() {
    let scratch = Scratch::new("kube_client/another-authority");
    let fake = FakeKube::start_untrusted(scratch.path());
    let error = fake
        .client()
        .list(INSTANCES, Kind::Pod, SELECTOR)
        .expect_err("refused");
    assert!(matches!(error, Error::Tls(_)), "a TLS error: {error:?}");
    assert!(fake.requests().is_empty(), "no request was sent");
}

#[test]
fn each_kind_is_created_read_listed_and_deleted_at_its_path() {
    let (_scratch, fake) = case("kinds");
    let client = fake.client();
    for (kind, resource) in [
        (Kind::Pod, "pods"),
        (Kind::PersistentVolumeClaim, "persistentvolumeclaims"),
        (Kind::Secret, "secrets"),
    ] {
        assert_eq!(kind.resource(), resource);
        let base = format!("/api/v1/namespaces/{INSTANCES}/{resource}");
        let mine = format!("conductor-alpha-{resource}");
        let created = client
            .create(INSTANCES, kind, &object(&mine, "alpha"))
            .expect("created");
        assert_eq!(created["metadata"]["name"], mine.as_str());
        assert_eq!(created["metadata"]["namespace"], INSTANCES);
        client
            .create(INSTANCES, kind, &other(&format!("foreign-{resource}")))
            .expect("created");
        let read = client.get(INSTANCES, kind, &mine).expect("read");
        assert_eq!(read["metadata"]["uid"], created["metadata"]["uid"]);
        let listed = client.list(INSTANCES, kind, SELECTOR).expect("listed");
        let names: Vec<&str> = listed
            .iter()
            .map(|item| item["metadata"]["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, [mine.as_str()], "only the selected objects");
        client.delete(INSTANCES, kind, &mine).expect("deleted");
        assert!(fake.object(INSTANCES, resource, &mine).is_none());

        let requests = fake.requests();
        let seen: Vec<(String, String, Option<String>)> = requests[requests.len() - 5..]
            .iter()
            .map(|r| {
                assert_eq!(r.bearer.as_deref(), Some(TOKEN), "{} {}", r.method, r.path);
                (
                    r.method.clone(),
                    r.path.clone(),
                    r.query.get("labelSelector").cloned(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                ("POST".to_owned(), base.clone(), None),
                ("POST".to_owned(), base.clone(), None),
                ("GET".to_owned(), format!("{base}/{mine}"), None),
                ("GET".to_owned(), base.clone(), Some(SELECTOR.to_owned())),
                ("DELETE".to_owned(), format!("{base}/{mine}"), None),
            ]
        );
        let sent = requests[requests.len() - 5].body.clone().expect("a body");
        assert_eq!(sent, object(&mine, "alpha"), "the object is sent as given");
    }
}

#[test]
fn not_found_and_conflict_are_typed() {
    let (_scratch, fake) = case("typed");
    let client = fake.client();
    let error = client
        .get(INSTANCES, Kind::Secret, "conductor-missing")
        .expect_err("absent");
    assert!(error.is_not_found() && !error.is_conflict(), "{error:?}");
    let Error::Api(status) = &error else {
        panic!("not an API status: {error:?}");
    };
    assert_eq!(
        (status.code, status.reason.as_str(), status.message.as_str()),
        (404, "NotFound", "secrets \"conductor-missing\" not found")
    );
    let error = client
        .delete(INSTANCES, Kind::Pod, "conductor-missing")
        .expect_err("absent");
    assert!(error.is_not_found(), "{error:?}");

    let claim = object("conductor-alpha-data", "alpha");
    client
        .create(INSTANCES, Kind::PersistentVolumeClaim, &claim)
        .expect("created");
    let error = client
        .create(INSTANCES, Kind::PersistentVolumeClaim, &claim)
        .expect_err("taken");
    assert!(error.is_conflict() && !error.is_not_found(), "{error:?}");
    let Error::Api(status) = &error else {
        panic!("not an API status: {error:?}");
    };
    assert_eq!(
        (status.code, status.reason.as_str()),
        (409, "AlreadyExists")
    );
    assert!(error.to_string().contains("already exists"), "{error}");

    fake.fail_next(403, "Forbidden", "pods is forbidden");
    let error = client
        .list(INSTANCES, Kind::Pod, SELECTOR)
        .expect_err("forbidden");
    assert!(
        matches!(&error, Error::Api(s) if s.code == 403),
        "{error:?}"
    );
}

#[test]
fn a_chunked_answer_is_decoded() {
    let (_scratch, fake) = case("chunked");
    let client = fake.client();
    let mut big = object("conductor-alpha", "alpha");
    big["data"] = json!({"conductor.yaml": "x".repeat(20_000)});
    client
        .create(INSTANCES, Kind::Secret, &big)
        .expect("created");
    fake.chunked(true);
    let read = client
        .get(INSTANCES, Kind::Secret, "conductor-alpha")
        .expect("a chunked answer decoded");
    assert_eq!(read["data"], big["data"]);
    let listed = client
        .list(INSTANCES, Kind::Secret, SELECTOR)
        .expect("a chunked list decoded");
    assert_eq!(listed.len(), 1);
    let error = client
        .get(INSTANCES, Kind::Secret, "conductor-beta")
        .expect_err("absent");
    assert!(error.is_not_found(), "a chunked Status decoded: {error:?}");
}

#[test]
fn a_call_ends_at_its_deadline() {
    let (_scratch, fake) = case("deadline");
    fake.stall(Duration::from_secs(5));
    let mut config = fake.config();
    config.deadline = Duration::from_millis(300);
    let client = conductor_cli::kube::Client::new(config).expect("a client");
    let started = Instant::now();
    let error = client
        .list(INSTANCES, Kind::Pod, SELECTOR)
        .expect_err("too late");
    let took = started.elapsed();
    assert!(matches!(error, Error::Timeout(_)), "a timeout: {error:?}");
    assert!(took < Duration::from_secs(2), "it waited {took:?}");
}

#[test]
fn a_refused_connection_is_its_own_error() {
    let (_scratch, fake) = case("refused");
    let closed = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = closed.local_addr().expect("address").port();
    drop(closed);
    let mut config = fake.config();
    config.port = port;
    let error = conductor_cli::kube::Client::new(config)
        .expect("a client")
        .get(INSTANCES, Kind::Pod, "conductor-alpha")
        .expect_err("nobody listens");
    assert!(
        matches!(error, Error::Connect(_)),
        "a connect error: {error:?}"
    );
}

#[test]
fn the_access_review_answers_whether_a_verb_is_allowed() {
    let (_scratch, fake) = case("review");
    fake.allow(INSTANCES, "create", "pods");
    fake.allow(INSTANCES, "get", "pods/log");
    let client = fake.client();
    assert!(client.may(INSTANCES, "create", "pods").expect("reviewed"));
    assert!(client.may(INSTANCES, "get", "pods/log").expect("reviewed"));
    assert!(
        !client
            .may(INSTANCES, "delete", "secrets")
            .expect("reviewed")
    );
    assert!(!client.may(NAMESPACE, "create", "pods").expect("reviewed"));
    let request = &fake.requests()[0];
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/apis/authorization.k8s.io/v1/selfsubjectaccessreviews"
    );
    let body = request.body.clone().expect("a review");
    assert_eq!(body["kind"], "SelfSubjectAccessReview");
    assert_eq!(body["apiVersion"], "authorization.k8s.io/v1");
    assert_eq!(
        body["spec"]["resourceAttributes"],
        json!({"namespace": INSTANCES, "verb": "create", "resource": "pods"})
    );
    let attributes =
        &fake.requests()[1].body.clone().expect("a review")["spec"]["resourceAttributes"];
    assert_eq!(attributes["resource"], "pods");
    assert_eq!(attributes["subresource"], "log");
}

#[test]
fn the_log_tail_reads_the_last_lines() {
    let (_scratch, fake) = case("log");
    fake.set_log(
        INSTANCES,
        "conductor-alpha",
        "starting\nreading config\nerror: config: instances[0].name: no DNS label\n",
    );
    let client = fake.client();
    let tail = client
        .log_tail(INSTANCES, "conductor-alpha", 2)
        .expect("the tail");
    assert_eq!(
        tail,
        "reading config\nerror: config: instances[0].name: no DNS label\n"
    );
    let request = &fake.requests()[0];
    assert_eq!(request.method, "GET");
    assert_eq!(
        request.path,
        format!("/api/v1/namespaces/{INSTANCES}/pods/conductor-alpha/log")
    );
    assert_eq!(
        request.query.get("tailLines").map(String::as_str),
        Some("2")
    );
    let error = client
        .log_tail(INSTANCES, "conductor-beta", 2)
        .expect_err("no such pod");
    assert!(error.is_not_found(), "{error:?}");
}

#[test]
fn a_pod_reads_as_its_phase_address_and_container_states() {
    let (_scratch, fake) = case("pod");
    let mut pod = object("conductor-alpha", "alpha");
    pod["metadata"]["deletionTimestamp"] = json!("2026-11-12T10:00:00Z");
    pod["status"] = json!({
        "phase": "Failed",
        "podIP": "10.42.0.7",
        "containerStatuses": [
            {"name": "runtime", "state": {"terminated": {
                "exitCode": 1, "reason": "Error", "message": "config: no DNS label"}}},
            {"name": "waiting", "state": {"waiting": {
                "reason": "ImagePullBackOff", "message": "pull access denied"}}},
            {"name": "running", "state": {"running": {"startedAt": "2026-11-12T09:00:00Z"}}},
        ],
    });
    fake.put(INSTANCES, "pods", pod);
    let listed = fake
        .client()
        .list(INSTANCES, Kind::Pod, SELECTOR)
        .expect("listed");
    let pod = Pod::of(&listed[0]);
    assert_eq!(pod.meta.name, "conductor-alpha");
    assert_eq!(pod.meta.labels["conductor/instance"], "alpha");
    assert_eq!(
        pod.meta.deletion_timestamp.as_deref(),
        Some("2026-11-12T10:00:00Z")
    );
    assert_eq!(pod.phase.as_deref(), Some("Failed"));
    assert_eq!(pod.ip.as_deref(), Some("10.42.0.7"));
    let states: Vec<(&str, &ContainerState)> = pod
        .containers
        .iter()
        .map(|c| (c.name.as_str(), &c.state))
        .collect();
    assert_eq!(
        states,
        [
            (
                "runtime",
                &ContainerState::Terminated {
                    exit_code: 1,
                    reason: Some("Error".to_owned()),
                    message: Some("config: no DNS label".to_owned()),
                }
            ),
            (
                "waiting",
                &ContainerState::Waiting {
                    reason: Some("ImagePullBackOff".to_owned()),
                    message: Some("pull access denied".to_owned()),
                }
            ),
            ("running", &ContainerState::Running),
        ]
    );
    let bare = Pod::of(&json!({"metadata": {"name": "conductor-beta"}}));
    assert_eq!(bare.phase, None);
    assert!(bare.containers.is_empty() && bare.meta.labels.is_empty());
}
