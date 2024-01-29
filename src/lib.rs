#![allow(dead_code)]
// Copyright (c) 2023 Murilo Ijanc' <mbsd@m0x.ru>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use k8s_openapi::api::core::v1::Pod;
use serde_json::json;

use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client,
};

async fn pod_crud() {
    let client = Client::try_default().await.unwrap();

    // Manage pods
    let pods: Api<Pod> = Api::default_namespaced(client);

    // Create Pod blog
    println!("Creating Pod instance blog");
    let p: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "blog" },
        "spec": {
            "containers": [{
              "name": "blog",
              "image": "clux/blog:0.1.0"
            }],
        }
    })).unwrap();

    let pp = PostParams::default();
    match pods.create(&pp, &p).await {
        Ok(o) => {
            let name = o.name_any();
            assert_eq!(p.name_any(), name);
            println!("Created {}", name);
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => panic!("Error creating pod {:?}",e),                        // any other case is probably bad
    }

    // Watch it phase for a few seconds
    let establish = await_condition(pods.clone(), "blog", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), establish).await.unwrap();

    // Verify we can get it
    println!("Get Pod blog");
    let p1cpy = pods.get("blog").await.unwrap();
    if let Some(spec) = &p1cpy.spec {
        println!("Got blog pod with containers: {:?}", spec.containers);
        assert_eq!(spec.containers[0].name, "blog");
    }

    // Replace its spec
    println!("Patch Pod blog");
    let patch = json!({
        "metadata": {
            "resourceVersion": p1cpy.resource_version(),
        },
        "spec": {
            "activeDeadlineSeconds": 5
        }
    });
    let patchparams = PatchParams::default();
    let p_patched = pods.patch("blog", &patchparams, &Patch::Merge(&patch)).await.unwrap();
    assert_eq!(p_patched.spec.unwrap().active_deadline_seconds, Some(5));

    let lp = ListParams::default().fields(&format!("metadata.name={}", "blog")); // only want results for our pod
    for p in pods.list(&lp).await.unwrap() {
        println!("Found Pod: {}", p.name_any());
    }

    // Delete it
    let dp = DeleteParams::default();
    pods.delete("blog", &dp).await.unwrap().map_left(|pdel| {
        assert_eq!(pdel.name_any(), "blog");
        println!("Deleting blog pod started: {:?}", pdel);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simple_test() {
        pod_crud().await;
        assert!(true);
    }
}
