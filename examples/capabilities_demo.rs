//! Kotoba Capabilities Demo
//!
//! この例はKotobaのcapabilityベースセキュリティシステムの実装方法を示します。
//! Denoに似た機能ベースのセキュリティモデルを提供します。

use kotoba_security::{
    capabilities::*,
    SecurityService, SecurityConfig, CapabilityConfig,
    Principal, Resource,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️  Kotoba Capabilities Demo");
    println!("===========================");

    // 機能設定を作成
    let capability_config = CapabilityConfig {
        enable_logging: true,
        enable_auditing: true,
        default_attenuation: None,
    };

    let security_config = SecurityConfig {
        capability_config,
        ..Default::default()
    };

    // セキュリティサービスを初期化
    let security_service = SecurityService::new(security_config)?;

    println!("✅ Capability-based security service initialized");

    // デモの実行
    run_capability_demos(&security_service).await?;

    println!("\n🎉 All capability demos completed successfully!");
    println!("\n📝 Key Benefits of Capability-Based Security:");
    println!("   • Fine-grained permissions instead of roles");
    println!("   • Explicit capability grants (no implicit permissions)");
    println!("   • Capability attenuation for safer operations");
    println!("   • Principle of least privilege enforcement");
    println!("   • Similar to Deno's permission model");

    Ok(())
}

async fn run_capability_demos(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Running Capability Demos");
    println!("==========================");

    // 1. 基本的な機能作成とチェック
    demo_basic_capabilities(security_service)?;

    // 2. 機能セットの操作
    demo_capability_sets(security_service)?;

    // 3. プリンシパルと認可
    demo_principals_and_authorization(security_service)?;

    // 4. 機能減衰（attenuation）
    demo_capability_attenuation(security_service)?;

    // 5. プリセット機能セット
    demo_preset_capability_sets(security_service)?;

    Ok(())
}

fn demo_basic_capabilities(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Demo 1: Basic Capabilities");
    println!("-----------------------------");

    // 基本的な機能を作成
    let read_users = Capability::new(ResourceType::Graph, Action::Read, Some("users:*".to_string()));
    let write_posts = Capability::new(ResourceType::Graph, Action::Write, Some("posts:owned".to_string()));
    let network_access = Capability::new(ResourceType::Network, Action::Read, Some("api:*".to_string()));

    println!("✅ Created capabilities:");
    println!("   • Read access to all users");
    println!("   • Write access to owned posts");
    println!("   • Read access to API endpoints");

    // 機能マッチングをテスト
    let test_resource = security_service.create_resource(
        ResourceType::Graph,
        Action::Read,
        Some("users:123".to_string()),
        std::collections::HashMap::new(),
    );

    let allowed = security_service.check_authorization(
        &Principal {
            user_id: "test-user".to_string(),
            roles: vec![],
            permissions: vec![],
            capabilities: {
                let mut cap_set = CapabilitySet::new();
                cap_set.add_capability(read_users.clone());
                cap_set
            },
            attributes: std::collections::HashMap::new(),
        },
        &test_resource,
    );

    println!("✅ Authorization check result: {}", if allowed.allowed { "ALLOWED" } else { "DENIED" });

    Ok(())
}

fn demo_capability_sets(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📚 Demo 2: Capability Sets");
    println!("-------------------------");

    // 機能セットを作成
    let mut admin_caps = CapabilitySet::new();
    admin_caps.add_capability(Capability::new(ResourceType::Graph, Action::Read, None));
    admin_caps.add_capability(Capability::new(ResourceType::Graph, Action::Write, None));
    admin_caps.add_capability(Capability::new(ResourceType::Graph, Action::Delete, None));

    let mut user_caps = CapabilitySet::new();
    user_caps.add_capability(Capability::new(ResourceType::Graph, Action::Read, Some("owned:*".to_string())));
    user_caps.add_capability(Capability::new(ResourceType::Graph, Action::Write, Some("owned:*".to_string())));

    println!("✅ Created capability sets:");
    println!("   • Admin set: {} capabilities", admin_caps.len());
    println!("   • User set: {} capabilities", user_caps.len());

    // 機能の統合
    let combined_caps = admin_caps.union(&user_caps);
    println!("✅ Combined set: {} capabilities", combined_caps.len());

    // 積集合
    let intersection_caps = admin_caps.intersection(&user_caps);
    println!("✅ Intersection: {} capabilities", intersection_caps.len());

    Ok(())
}

fn demo_principals_and_authorization(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n👤 Demo 3: Principals and Authorization");
    println!("--------------------------------------");

    // 管理者プリンシパルを作成
    let admin_principal = security_service.create_principal_with_capabilities(
        "admin-user".to_string(),
        {
            let mut caps = CapabilitySet::new();
            caps.add_capability(Capability::new(ResourceType::Graph, Action::Admin, None));
            caps.add_capability(Capability::new(ResourceType::User, Action::Admin, None));
            caps
        },
        vec!["admin".to_string()],
        vec!["system:*".to_string()],
        std::collections::HashMap::new(),
    );

    // コンテンツ作成者プリンシパルを作成
    let creator_principal = security_service.create_principal_with_capabilities(
        "content-creator".to_string(),
        {
            let mut caps = CapabilitySet::new();
            caps.add_capability(Capability::new(ResourceType::Graph, Action::Read, Some("posts:*".to_string())));
            caps.add_capability(Capability::new(ResourceType::Graph, Action::Write, Some("posts:owned".to_string())));
            caps
        },
        vec!["creator".to_string()],
        vec!["content:*".to_string()],
        std::collections::HashMap::new(),
    );

    println!("✅ Created principals:");
    println!("   • Admin: {} capabilities", admin_principal.capabilities.len());
    println!("   • Creator: {} capabilities", creator_principal.capabilities.len());

    // 認可チェック
    let admin_resource = security_service.create_resource(
        ResourceType::User,
        Action::Admin,
        None,
        std::collections::HashMap::new(),
    );

    let admin_allowed = security_service.check_authorization(&admin_principal, &admin_resource);
    let creator_allowed = security_service.check_authorization(&creator_principal, &admin_resource);

    println!("✅ Admin user can manage users: {}", admin_allowed.allowed);
    println!("✅ Creator user can manage users: {}", creator_allowed.allowed);

    Ok(())
}

fn demo_capability_attenuation(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔒 Demo 4: Capability Attenuation");
    println!("--------------------------------");

    // 広範な機能を作成
    let broad_cap = Capability::new(ResourceType::Graph, Action::Read, None);
    println!("✅ Created broad capability: Read all graphs");

    // 機能を減衰（制限）
    let attenuated_cap = broad_cap.attenuate(Some("users:*".to_string()));
    println!("✅ Attenuated to: Read only user graphs");

    // 機能セットを作成して減衰
    let mut original_set = CapabilitySet::new();
    original_set.add_capability(Capability::new(ResourceType::Graph, Action::Write, None));
    original_set.add_capability(Capability::new(ResourceType::FileSystem, Action::Read, None));

    // 制限を定義
    let restrictions = vec![
        Capability::new(ResourceType::Graph, Action::Write, Some("safe:*".to_string())),
        Capability::new(ResourceType::FileSystem, Action::Read, Some("/tmp/*".to_string())),
    ];

    let attenuated_set = security_service.attenuate_capabilities(&original_set, restrictions);

    println!("✅ Original set: {} capabilities", original_set.len());
    println!("✅ Attenuated set: {} capabilities", attenuated_set.len());
    println!("✅ Attenuation provides safer, more restricted capabilities");

    Ok(())
}

fn demo_preset_capability_sets(security_service: &SecurityService) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Demo 5: Preset Capability Sets");
    println!("--------------------------------");

    // プリセット機能セットを作成
    let readonly_caps = CapabilityService::create_preset_capability_set(PresetCapabilitySet::ReadOnly);
    let readwrite_caps = CapabilityService::create_preset_capability_set(PresetCapabilitySet::ReadWrite);
    let admin_caps = CapabilityService::create_preset_capability_set(PresetCapabilitySet::Admin);

    println!("✅ Created preset capability sets:");
    println!("   • ReadOnly: {} capabilities", readonly_caps.len());
    println!("   • ReadWrite: {} capabilities", readwrite_caps.len());
    println!("   • Admin: {} capabilities", admin_caps.len());

    // 各プリセットの内容を表示
    println!("\n📋 ReadOnly capabilities:");
    for cap in &readonly_caps.capabilities {
        println!("   • {:?}::{:?} on {:?}", cap.resource_type, cap.action, cap.scope);
    }

    println!("\n📋 Admin capabilities:");
    for cap in &admin_caps.capabilities {
        println!("   • {:?}::{:?} on {:?}", cap.resource_type, cap.action, cap.scope);
    }

    Ok(())
}
