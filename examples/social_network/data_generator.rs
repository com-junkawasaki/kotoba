//! Social Networkデータジェネレータ
//!
//! ソーシャルネットワークのテストデータを生成するモジュール

use kotoba::*;
use std::collections::HashMap;

/// ソーシャルネットワークのデータ構造
#[derive(Debug, Clone)]
pub struct SocialNetwork {
    pub graph: GraphRef,
    pub users: Vec<User>,
    pub posts: Vec<Post>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: VertexId,
    pub name: String,
    pub age: u32,
    pub location: String,
    pub interests: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: VertexId,
    pub author_id: VertexId,
    pub content: String,
    pub timestamp: u64,
    pub likes: u32,
}

/// Social Networkデータジェネレータ
pub struct SocialNetworkGenerator;

impl SocialNetworkGenerator {
    /// 指定サイズのソーシャルネットワークを生成
    pub fn generate_network(user_count: usize, friendship_density: f64, post_count: usize) -> SocialNetwork {
        let mut graph = Graph::empty();
        let mut users = Vec::new();
        let mut posts = Vec::new();

        // ユーザーの生成
        println!("Generating {} users...", user_count);
        for i in 0..user_count {
            let user = Self::generate_user(&mut graph, i);
            users.push(user);
        }

        // 友人関係の生成
        println!("Generating friendships with density {:.2}...", friendship_density);
        Self::generate_friendships(&mut graph, &users, friendship_density);

        // 投稿の生成
        println!("Generating {} posts...", post_count);
        for _ in 0..post_count {
            let post = Self::generate_post(&mut graph, &users);
            posts.push(post);
        }

        // いいね関係の生成
        println!("Generating likes...");
        Self::generate_likes(&mut graph, &users, &posts);

        SocialNetwork {
            graph: GraphRef::new(graph),
            users,
            posts,
        }
    }

    /// ユーザーを生成
    fn generate_user(graph: &mut Graph, index: usize) -> User {
        let names = vec![
            "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Henry",
            "Ivy", "Jack", "Kate", "Liam", "Mia", "Noah", "Olivia", "Peter"
        ];

        let locations = vec![
            "Tokyo", "New York", "London", "Paris", "Berlin", "Sydney", "Toronto", "Singapore"
        ];

        let interests = vec![
            "Music", "Sports", "Technology", "Art", "Travel", "Food", "Movies", "Books"
        ];

        let name = format!("{} {}", names[index % names.len()], (index / names.len()) + 1);
        let age = (20 + (index % 50)) as u32;
        let location = locations[index % locations.len()].to_string();
        let user_interests = interests.iter()
            .enumerate()
            .filter(|(i, _)| index % (i + 2) == 0)
            .map(|(_, interest)| interest.to_string())
            .collect();

        let vertex = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec!["Person".to_string()],
            props: HashMap::from([
                ("name".to_string(), Value::String(name.clone())),
                ("age".to_string(), Value::Int(age as i64)),
                ("location".to_string(), Value::String(location.clone())),
                ("interests".to_string(), Value::String(user_interests.join(","))),
            ]),
        });

        User {
            id: vertex,
            name,
            age,
            location,
            interests: user_interests,
        }
    }

    /// 友人関係を生成
    fn generate_friendships(graph: &mut Graph, users: &[User], density: f64) {
        let friendship_count = ((users.len() as f64 * density) as usize).min(users.len() * (users.len() - 1) / 2);

        for i in 0..friendship_count {
            let user1_idx = i % users.len();
            let user2_idx = (i * 7 + 13) % users.len(); // 擬似乱数生成

            if user1_idx != user2_idx {
                // 双方向の友人関係を作成
                graph.add_edge(EdgeData {
                    id: uuid::Uuid::new_v4(),
                    src: users[user1_idx].id,
                    dst: users[user2_idx].id,
                    label: "FOLLOWS".to_string(),
                    props: HashMap::new(),
                });

                graph.add_edge(EdgeData {
                    id: uuid::Uuid::new_v4(),
                    src: users[user2_idx].id,
                    dst: users[user1_idx].id,
                    label: "FOLLOWS".to_string(),
                    props: HashMap::new(),
                });
            }
        }
    }

    /// 投稿を生成
    fn generate_post(graph: &mut Graph, users: &[User]) -> Post {
        let contents = vec![
            "Hello world!", "Having a great day!", "Just finished an amazing project",
            "Beautiful weather today", "Excited for the weekend!", "Learning something new",
            "Great conversation with friends", "Amazing food today", "Working on something exciting",
            "Love this new technology", "Inspiring day!", "Making progress every day"
        ];

        let author_idx = rand::random::<usize>() % users.len();
        let author = &users[author_idx];
        let content = contents[rand::random::<usize>() % contents.len()].to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - (rand::random::<u64>() % (30 * 24 * 60 * 60)); // 過去30日以内
        let likes = rand::random::<u32>() % 100;

        let vertex = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec!["Post".to_string()],
            props: HashMap::from([
                ("content".to_string(), Value::String(content.clone())),
                ("timestamp".to_string(), Value::Int(timestamp as i64)),
                ("likes".to_string(), Value::Int(likes as i64)),
            ]),
        });

        // 投稿者との関係を作成
        graph.add_edge(EdgeData {
            id: uuid::Uuid::new_v4(),
            src: author.id,
            dst: vertex,
            label: "POSTED".to_string(),
            props: HashMap::new(),
        });

        Post {
            id: vertex,
            author_id: author.id,
            content,
            timestamp,
            likes,
        }
    }

    /// いいね関係を生成
    fn generate_likes(graph: &mut Graph, users: &[User], posts: &[Post]) {
        for post in posts {
            let like_count = (post.likes as usize).min(users.len());
            let mut liked_users = std::collections::HashSet::new();

            for _ in 0..like_count {
                let user_idx = rand::random::<usize>() % users.len();
                if liked_users.insert(user_idx) {
                    graph.add_edge(EdgeData {
                        id: uuid::Uuid::new_v4(),
                        src: users[user_idx].id,
                        dst: post.id,
                        label: "LIKES".to_string(),
                        props: HashMap::new(),
                    });
                }
            }
        }
    }
}

/// 統計情報
impl SocialNetwork {
    /// ネットワークの統計情報を表示
    pub fn print_statistics(&self) {
        let graph = self.graph.read();

        println!("\n=== Social Network Statistics ===");
        println!("Users: {}", self.users.len());
        println!("Posts: {}", self.posts.len());
        println!("Vertices: {}", graph.vertex_count());
        println!("Edges: {}", graph.edge_count());

        // ユーザーラベル別のカウント
        let person_count = graph.vertices_by_label("Person").len();
        let post_count = graph.vertices_by_label("Post").len();
        println!("Person nodes: {}", person_count);
        println!("Post nodes: {}", post_count);

        // エッジラベル別のカウント
        let follows_count = graph.edges_by_label("FOLLOWS").len();
        let posted_count = graph.edges_by_label("POSTED").len();
        let likes_count = graph.edges_by_label("LIKES").len();
        println!("FOLLOWS edges: {}", follows_count);
        println!("POSTED edges: {}", posted_count);
        println!("LIKES edges: {}", likes_count);

        // 平均次数
        let total_degree: usize = self.users.iter()
            .map(|user| graph.degree(&user.id))
            .sum();
        let avg_degree = total_degree as f64 / self.users.len() as f64;
        println!("Average degree: {:.2}", avg_degree);

        println!("================================\n");
    }
}
