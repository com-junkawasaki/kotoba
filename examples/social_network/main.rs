//! Social Networkユースケースのデモンストレーション
//!
//! このプログラムはKotobaを使用したソーシャルネットワークの
//! 典型的なユースケースを実演します。

use kotoba::examples::social_network::*;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Kotoba Social Network Demo");
    println!("=============================\n");

    // ネットワークの生成
    println!("Generating social network...");
    let start_time = Instant::now();
    let network = SocialNetworkGenerator::generate_network(100, 0.1, 200);
    let generation_time = start_time.elapsed();
    println!("✅ Network generated in {:.2}ms", generation_time.as_millis());

    // 統計情報の表示
    network.print_statistics();

    // クエリ実行器の初期化
    let queries = SocialNetworkQueries::new();

    // ユースケース1: 友人関係の探索
    println!("🔍 Use Case 1: Finding Friends");
    println!("------------------------------");
    if let Some(first_user) = network.users.first() {
        let friends = queries.get_user_friends(&network, &first_user.name)?;
        println!("{}'s friends: {:?}", first_user.name, friends);
    }
    println!();

    // ユースケース2: タイムラインの取得
    println!("📱 Use Case 2: User Timeline");
    println!("----------------------------");
    if let Some(first_user) = network.users.first() {
        let timeline = queries.get_user_timeline(&network, &first_user.name, 5)?;
        println!("{}'s timeline (top 5):", first_user.name);
        for post in timeline {
            println!("  📝 {} ({} likes) - by {}", post.content, post.likes, post.author);
        }
    }
    println!();

    // ユースケース3: 共通の友人の検索
    println!("🤝 Use Case 3: Mutual Friends");
    println!("-----------------------------");
    if network.users.len() >= 2 {
        let user1 = &network.users[0].name;
        let user2 = &network.users[1].name;
        let mutual_friends = queries.find_mutual_friends(&network, user1, user2)?;
        println!("Mutual friends between {} and {}: {:?}", user1, user2, mutual_friends);
    }
    println!();

    // ユースケース4: 人気の投稿
    println!("🔥 Use Case 4: Popular Posts");
    println!("----------------------------");
    let popular_posts = queries.get_popular_posts(&network, 10, 5)?;
    println!("Popular posts (10+ likes, top 5):");
    for post in popular_posts {
        println!("  🔥 {} ({} likes) - by {}", post.content, post.likes, post.author);
    }
    println!();

    // ユースケース5: 興味に基づくおすすめユーザー
    println!("💡 Use Case 5: Interest-Based Recommendations");
    println!("--------------------------------------------");
    if let Some(first_user) = network.users.first() {
        let recommendations = queries.recommend_users_by_interests(&network, &first_user.name, 3)?;
        println!("Users recommended for {}: {:?}", first_user.name, recommendations);
    }
    println!();

    // ユースケース6: 場所別ユーザー検索
    println!("📍 Use Case 6: Location-Based Search");
    println!("------------------------------------");
    let tokyo_users = queries.find_users_by_location(&network, "Tokyo")?;
    println!("Users in Tokyo (top 5):");
    for user in tokyo_users.into_iter().take(5) {
        println!("  👤 {} (age {}) - interests: {:?}", user.name, user.age, user.interests);
    }
    println!();

    // 分析機能
    println!("📊 Network Analysis");
    println!("==================");

    // ネットワーク健全性のチェック
    let health_report = SocialNetworkAnalyzer::check_network_health(&network);
    health_report.print();

    // 中心性の計算
    let centrality = SocialNetworkAnalyzer::calculate_degree_centrality(&network);
    println!("🏆 Top 5 Most Central Users:");
    let mut centrality_vec: Vec<_> = centrality.into_iter().collect();
    centrality_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, score) in centrality_vec.into_iter().take(5) {
        println!("  {:.3} - {}", score, name);
    }
    println!();

    // クラスタリング係数
    let clustering = SocialNetworkAnalyzer::calculate_clustering_coefficient(&network);
    let avg_clustering: f64 = clustering.values().sum::<f64>() / clustering.len() as f64;
    println!("🔗 Average Clustering Coefficient: {:.4}", avg_clustering);
    println!();

    // コミュニティ検出
    let communities = SocialNetworkAnalyzer::detect_communities(&network);
    println!("👥 Detected Communities: {}", communities.len());
    for (i, community) in communities.iter().enumerate() {
        println!("  Community {}: {} users", i + 1, community.len());
    }
    println!();

    // 次数分布
    let degree_dist = SocialNetworkAnalyzer::analyze_degree_distribution(&network);
    println!("📈 Degree Distribution:");
    let mut sorted_degrees: Vec<_> = degree_dist.into_iter().collect();
    sorted_degrees.sort_by_key(|(k, _)| *k);
    for (degree, count) in sorted_degrees {
        println!("  Degree {}: {} users", degree, count);
    }
    println!();

    // インフルエンサー特定
    let influencers = SocialNetworkAnalyzer::identify_influencers(&network, 3);
    println!("⭐ Top 3 Influencers:");
    for (name, degree) in influencers {
        println!("  {} (degree: {})", name, degree);
    }
    println!();

    // ネットワーク密度
    let density = SocialNetworkAnalyzer::calculate_network_density(&network);
    println!("🌐 Network Density: {:.6}", density);
    println!();

    // 平均経路長（小規模ネットワークのみ）
    if network.users.len() <= 20 {
        let avg_path = queries.calculate_graph_diameter(&network, 10)?;
        println!("📏 Graph Diameter (max depth 10): {}", avg_path);
        println!();
    }

    println!("🎉 Demo completed successfully!");
    println!("==============================");

    Ok(())
}
