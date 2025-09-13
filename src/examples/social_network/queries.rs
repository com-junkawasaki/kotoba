//! Social Networkクエリ
//!
//! ソーシャルネットワークの典型的なクエリを実装

use crate::examples::social_network::*;
use crate::*;

/// ソーシャルネットワーククエリ実行器
pub struct SocialNetworkQueries {
    executor: QueryExecutor,
    catalog: Catalog,
}

impl SocialNetworkQueries {
    pub fn new() -> Self {
        Self {
            executor: QueryExecutor::new(),
            catalog: Catalog::empty(),
        }
    }

    /// ユーザーの友人一覧を取得
    pub fn get_user_friends(&self, network: &SocialNetwork, user_name: &str) -> Result<Vec<String>> {
        let gql = format!(r#"
            MATCH (u:Person)-[:FOLLOWS]->(f:Person)
            WHERE u.name = "{}"
            RETURN f.name as friend_name
            ORDER BY f.name
        "#, user_name);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut friends = Vec::new();
        for row in results {
            if let Some(Value::String(name)) = row.values.get("friend_name") {
                friends.push(name.clone());
            }
        }

        Ok(friends)
    }

    /// ユーザーのタイムラインを取得
    pub fn get_user_timeline(&self, network: &SocialNetwork, user_name: &str, limit: usize) -> Result<Vec<PostInfo>> {
        let gql = format!(r#"
            MATCH (u:Person)-[:FOLLOWS]->(f:Person)-[:POSTED]->(p:Post)
            WHERE u.name = "{}"
            RETURN p.content as content, p.timestamp as timestamp, p.likes as likes, f.name as author
            ORDER BY p.timestamp DESC
            LIMIT {}
        "#, user_name, limit);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut posts = Vec::new();
        for row in results {
            if let (Some(Value::String(content)), Some(Value::Int(timestamp)), Some(Value::Int(likes)), Some(Value::String(author))) =
               (row.values.get("content"), row.values.get("timestamp"), row.values.get("likes"), row.values.get("author")) {
                posts.push(PostInfo {
                    content: content.clone(),
                    timestamp: *timestamp as u64,
                    likes: *likes as u32,
                    author: author.clone(),
                });
            }
        }

        Ok(posts)
    }

    /// 共通の友人を検索
    pub fn find_mutual_friends(&self, network: &SocialNetwork, user1_name: &str, user2_name: &str) -> Result<Vec<String>> {
        let gql = format!(r#"
            MATCH (u1:Person)-[:FOLLOWS]->(f:Person)<-[:FOLLOWS]-(u2:Person)
            WHERE u1.name = "{}" AND u2.name = "{}" AND u1 <> u2
            RETURN f.name as mutual_friend
            ORDER BY f.name
        "#, user1_name, user2_name);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut mutual_friends = Vec::new();
        for row in results {
            if let Some(Value::String(name)) = row.values.get("mutual_friend") {
                mutual_friends.push(name.clone());
            }
        }

        Ok(mutual_friends)
    }

    /// 人気の投稿を取得
    pub fn get_popular_posts(&self, network: &SocialNetwork, min_likes: u32, limit: usize) -> Result<Vec<PostInfo>> {
        let gql = format!(r#"
            MATCH (p:Post)<-[:POSTED]-(u:Person)
            WHERE p.likes >= {}
            RETURN p.content as content, p.timestamp as timestamp, p.likes as likes, u.name as author
            ORDER BY p.likes DESC, p.timestamp DESC
            LIMIT {}
        "#, min_likes, limit);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut posts = Vec::new();
        for row in results {
            if let (Some(Value::String(content)), Some(Value::Int(timestamp)), Some(Value::Int(likes)), Some(Value::String(author))) =
               (row.values.get("content"), row.values.get("timestamp"), row.values.get("likes"), row.values.get("author")) {
                posts.push(PostInfo {
                    content: content.clone(),
                    timestamp: *timestamp as u64,
                    likes: *likes as u32,
                    author: author.clone(),
                });
            }
        }

        Ok(posts)
    }

    /// ユーザーの興味に基づくおすすめユーザーを検索
    pub fn recommend_users_by_interests(&self, network: &SocialNetwork, user_name: &str, limit: usize) -> Result<Vec<String>> {
        let gql = format!(r#"
            MATCH (u:Person), (other:Person)
            WHERE u.name = "{}" AND other.name <> "{}" AND
                  size(split(u.interests, ",")) > 0 AND
                  size(split(other.interests, ",")) > 0
            WITH u, other,
                 [x for x in split(u.interests, ",") if contains(other.interests, x)] as common_interests
            WHERE size(common_interests) > 0
            RETURN other.name as recommended_user, size(common_interests) as common_count
            ORDER BY common_count DESC, other.name
            LIMIT {}
        "#, user_name, user_name, limit);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut recommendations = Vec::new();
        for row in results {
            if let Some(Value::String(name)) = row.values.get("recommended_user") {
                recommendations.push(name.clone());
            }
        }

        Ok(recommendations)
    }

    /// 特定の場所のユーザーを検索
    pub fn find_users_by_location(&self, network: &SocialNetwork, location: &str) -> Result<Vec<UserInfo>> {
        let gql = format!(r#"
            MATCH (u:Person)
            WHERE u.location = "{}"
            RETURN u.name as name, u.age as age, u.interests as interests
            ORDER BY u.age, u.name
        "#, location);

        let results = self.executor.execute_gql(&gql, &network.graph, &self.catalog)?;

        let mut users = Vec::new();
        for row in results {
            if let (Some(Value::String(name)), Some(Value::Int(age)), Some(Value::String(interests))) =
               (row.values.get("name"), row.values.get("age"), row.values.get("interests")) {
                users.push(UserInfo {
                    name: name.clone(),
                    age: *age as u32,
                    interests: interests.split(',').map(|s| s.to_string()).collect(),
                });
            }
        }

        Ok(users)
    }

    /// ソーシャルグラフの直径を計算（簡易版）
    pub fn calculate_graph_diameter(&self, network: &SocialNetwork, max_depth: usize) -> Result<usize> {
        let mut max_distance = 0;
        let graph = network.graph.read();

        // 各ユーザーからの最長経路を計算（簡易実装）
        for user in &network.users {
            let mut visited = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();

            visited.insert(user.id);
            queue.push_back((user.id, 0));

            let mut local_max = 0;

            while let Some((current, distance)) = queue.pop_front() {
                local_max = local_max.max(distance);

                if distance >= max_depth {
                    continue;
                }

                // 友人関係をたどる
                for neighbor in graph.adj_out.get(&current).unwrap_or(&std::collections::HashSet::new()) {
                    if !visited.contains(neighbor) {
                        visited.insert(*neighbor);
                        queue.push_back((*neighbor, distance + 1));
                    }
                }
            }

            max_distance = max_distance.max(local_max);
        }

        Ok(max_distance)
    }
}

/// クエリ結果用の構造体
#[derive(Debug, Clone)]
pub struct PostInfo {
    pub content: String,
    pub timestamp: u64,
    pub likes: u32,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub name: String,
    pub age: u32,
    pub interests: Vec<String>,
}
