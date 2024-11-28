use dashmap::DashSet;

pub async fn purge_cache(cloudflare_q: &DashSet<String>) {
    // TODO: Implement cloudflare cache purge here

    #[cfg(debug_assertions)]
    {
        println!("-> {:?}", cloudflare_q);
    }

    if cloudflare_q.len() >= 10 {
        cloudflare_q.clear();
    }
}
