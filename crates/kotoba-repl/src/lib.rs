    #[cfg(test)]
    mod tests {

        #[tokio::test]
        async fn test_repl_basic_commands() {
            let config = crate::ReplConfig::default();
            let mut session = crate::ReplSession::new(config);

            // 変数宣言のテスト
            let result = session.execute("let x = 42").await.unwrap();
            assert!(result.is_success());
            assert!(result.output.is_some());

            // ヘルプコマンドのテスト
            let help_result = session.execute(".help").await.unwrap();
            assert!(help_result.is_success());
            assert!(help_result.output.is_some());
            assert!(help_result.output.as_ref().unwrap().contains("Kotoba REPL Commands"));

            // セッション情報のテスト
            let info = session.get_info();
            assert_eq!(info.command_count, 2);
        }

        #[tokio::test]
        async fn test_variable_operations() {
            let config = crate::ReplConfig::default();
            let mut session = crate::ReplSession::new(config);

            // 変数宣言
            let result1 = session.execute("let name = \"Alice\"").await.unwrap();
            assert!(result1.is_success());

            // 変数一覧表示
            let result2 = session.execute(".vars").await.unwrap();
            assert!(result2.is_success());
            assert!(result2.output.as_ref().unwrap().contains("name"));
            assert!(result2.output.as_ref().unwrap().contains("Alice"));
        }

        #[tokio::test]
        async fn test_expression_evaluation() {
            let config = crate::ReplConfig::default();
            let mut session = crate::ReplSession::new(config);

            // 簡単な式の評価
            let result = session.execute("1 + 2").await.unwrap();
            assert!(result.is_success());
            // 簡易的な評価なので、結果は実行されたことを示すメッセージになる
        }
    }