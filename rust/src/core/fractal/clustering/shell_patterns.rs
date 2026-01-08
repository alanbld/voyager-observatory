//! Shell Pattern Recognition
//!
//! Identifies common shell script patterns for semantic clustering.
//! Recognizes deployment, error handling, data processing, and automation patterns.

// Note: HashMap may be used for pattern caching in future

use regex::Regex;
use serde::{Deserialize, Serialize};

// =============================================================================
// Pattern Types
// =============================================================================

/// Types of shell script patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellPatternType {
    /// Deployment scripts (docker, k8s, terraform)
    Deployment,
    /// Error handling (set -e, trap)
    ErrorHandling,
    /// Data processing (awk, sed, jq)
    DataProcessing,
    /// Automation (cron, systemd)
    Automation,
    /// Backup operations
    Backup,
    /// Monitoring and logging
    Monitoring,
    /// Testing scripts
    Testing,
    /// Build scripts
    Build,
    /// Cleanup operations
    Cleanup,
    /// Setup and initialization
    Setup,
    /// Network operations
    Network,
    /// Security-related
    Security,
    /// Unknown pattern
    Unknown,
}

impl ShellPatternType {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Deployment => "Deployment and release automation",
            Self::ErrorHandling => "Error handling and failure recovery",
            Self::DataProcessing => "Data transformation and processing",
            Self::Automation => "Scheduled tasks and automation",
            Self::Backup => "Backup and restore operations",
            Self::Monitoring => "Monitoring, logging, and alerting",
            Self::Testing => "Testing and validation",
            Self::Build => "Build and compilation",
            Self::Cleanup => "Cleanup and maintenance",
            Self::Setup => "Setup and initialization",
            Self::Network => "Network operations",
            Self::Security => "Security operations",
            Self::Unknown => "Unknown pattern",
        }
    }
}

// =============================================================================
// Pattern Result
// =============================================================================

/// A recognized shell pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPattern {
    /// Type of pattern
    pub pattern_type: ShellPatternType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Specific indicators found
    pub indicators: Vec<String>,
    /// Tools detected
    pub tools: Vec<String>,
    /// Stages/phases detected
    pub stages: Vec<String>,
}

impl ShellPattern {
    pub fn new(pattern_type: ShellPatternType, confidence: f32) -> Self {
        Self {
            pattern_type,
            confidence,
            indicators: Vec::new(),
            tools: Vec::new(),
            stages: Vec::new(),
        }
    }

    pub fn with_indicator(mut self, indicator: impl Into<String>) -> Self {
        self.indicators.push(indicator.into());
        self
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stages.push(stage.into());
        self
    }
}

// =============================================================================
// Shell Pattern Recognizer
// =============================================================================

/// Recognizes patterns in shell scripts.
pub struct ShellPatternRecognizer {
    // Pattern matchers
    deployment_patterns: Vec<Regex>,
    error_patterns: Vec<Regex>,
    data_patterns: Vec<Regex>,
    automation_patterns: Vec<Regex>,
    backup_patterns: Vec<Regex>,
    monitoring_patterns: Vec<Regex>,
    testing_patterns: Vec<Regex>,
    build_patterns: Vec<Regex>,
    cleanup_patterns: Vec<Regex>,
    setup_patterns: Vec<Regex>,
    network_patterns: Vec<Regex>,
    security_patterns: Vec<Regex>,
}

impl Default for ShellPatternRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellPatternRecognizer {
    pub fn new() -> Self {
        Self {
            deployment_patterns: vec![
                Regex::new(r"(?i)docker\s+(build|push|pull|run|compose)").unwrap(),
                Regex::new(r"(?i)kubectl\s+(apply|delete|rollout|get|describe)").unwrap(),
                Regex::new(r"(?i)helm\s+(install|upgrade|uninstall|template)").unwrap(),
                Regex::new(r"(?i)terraform\s+(apply|plan|destroy|init)").unwrap(),
                Regex::new(r"(?i)ansible(-playbook)?").unwrap(),
                Regex::new(r"(?i)aws\s+(s3|ec2|ecs|lambda|deploy)").unwrap(),
                Regex::new(r"(?i)gcloud\s+(deploy|run)").unwrap(),
                Regex::new(r"(?i)az\s+(webapp|functionapp|container)").unwrap(),
                Regex::new(r"(?i)(deploy|release|rollout|publish)\s+").unwrap(),
            ],

            error_patterns: vec![
                Regex::new(r"set\s+-[euo]+").unwrap(),
                Regex::new(r"set\s+-o\s+(errexit|nounset|pipefail)").unwrap(),
                Regex::new(r#"trap\s+['"].*?['"]\s+(EXIT|ERR|INT|TERM)"#).unwrap(),
                Regex::new(r"\|\|\s*(exit|return|die|fatal)").unwrap(),
                Regex::new(r"if\s+\[\s*\$\?\s*-ne\s*0\s*\]").unwrap(),
                Regex::new(r"(?i)(handle|catch|recover).*error").unwrap(),
            ],

            data_patterns: vec![
                Regex::new(r"(?i)\bawk\s+").unwrap(),
                Regex::new(r"(?i)\bsed\s+").unwrap(),
                Regex::new(r"(?i)\bjq\s+").unwrap(),
                Regex::new(r"(?i)\bgrep\s+").unwrap(),
                Regex::new(r"(?i)\bcut\s+").unwrap(),
                Regex::new(r"(?i)\bsort\s+").unwrap(),
                Regex::new(r"(?i)\buniq\s+").unwrap(),
                Regex::new(r"(?i)\bxargs\s+").unwrap(),
                Regex::new(r"(?i)\btr\s+").unwrap(),
                Regex::new(r"\|\s*while\s+read").unwrap(),
            ],

            automation_patterns: vec![
                Regex::new(r"(?i)cron(tab)?").unwrap(),
                Regex::new(r"#\s*@(hourly|daily|weekly|monthly|yearly)").unwrap(),
                Regex::new(r"\d+\s+\d+\s+\*\s+\*\s+\*").unwrap(), // cron expression
                Regex::new(r"(?i)systemd|systemctl").unwrap(),
                Regex::new(r"(?i)\bat\s+").unwrap(),
                Regex::new(r"(?i)\bwatch\s+").unwrap(),
                Regex::new(r"while\s+true\s*;?\s*do").unwrap(),
                Regex::new(r"(?i)schedule|scheduled").unwrap(),
            ],

            backup_patterns: vec![
                Regex::new(r"(?i)\btar\s+[a-z]*[czx]").unwrap(),
                Regex::new(r"(?i)\brsync\s+").unwrap(),
                Regex::new(r"(?i)\bcp\s+-[ra]").unwrap(),
                Regex::new(r"(?i)\bdd\s+").unwrap(),
                Regex::new(r"(?i)(backup|bak|snapshot|archive)").unwrap(),
                Regex::new(r"(?i)pg_dump|mysqldump|mongodump").unwrap(),
            ],

            monitoring_patterns: vec![
                Regex::new(r"(?i)\blog\s").unwrap(),
                Regex::new(r"(?i)logger\s+").unwrap(),
                Regex::new(r"(?i)syslog").unwrap(),
                Regex::new(r"(?i)(prometheus|grafana|datadog|newrelic)").unwrap(),
                Regex::new(r"(?i)health.*check").unwrap(),
                Regex::new(r"(?i)(alert|notify|slack|email)").unwrap(),
                Regex::new(r"(?i)\btop\s|\bhtop\s|\bvmstat\s").unwrap(),
            ],

            testing_patterns: vec![
                Regex::new(r"(?i)\btest\s+").unwrap(),
                Regex::new(r"(?i)(pytest|jest|mocha|rspec|unittest)").unwrap(),
                Regex::new(r"(?i)assert|expect").unwrap(),
                Regex::new(r"(?i)mock|stub|fake").unwrap(),
                Regex::new(r"(?i)(integration|unit|e2e).*test").unwrap(),
                Regex::new(r"\[\s*-[a-z]\s+").unwrap(), // test expressions
            ],

            build_patterns: vec![
                Regex::new(r"(?i)\bmake\s+").unwrap(),
                Regex::new(r"(?i)\bcmake\s+").unwrap(),
                Regex::new(r"(?i)\bcargo\s+(build|test|run)").unwrap(),
                Regex::new(r"(?i)\bnpm\s+(run|build|install)").unwrap(),
                Regex::new(r"(?i)\byarn\s+(build|install)").unwrap(),
                Regex::new(r"(?i)\bgo\s+(build|test)").unwrap(),
                Regex::new(r"(?i)\bmvn\s+").unwrap(),
                Regex::new(r"(?i)\bgradle\s+").unwrap(),
                Regex::new(r"(?i)(compile|build|package)").unwrap(),
            ],

            cleanup_patterns: vec![
                Regex::new(r"(?i)\brm\s+-[rf]+").unwrap(),
                Regex::new(r"(?i)\bfind\s+.*-delete").unwrap(),
                Regex::new(r"(?i)(cleanup|clean|purge|prune)").unwrap(),
                Regex::new(r"(?i)docker\s+(system\s+)?prune").unwrap(),
                Regex::new(r"(?i)apt(-get)?\s+autoremove").unwrap(),
            ],

            setup_patterns: vec![
                Regex::new(r"(?i)(init|setup|install|bootstrap)").unwrap(),
                Regex::new(r"(?i)apt(-get)?\s+install").unwrap(),
                Regex::new(r"(?i)yum\s+install").unwrap(),
                Regex::new(r"(?i)pip\s+install").unwrap(),
                Regex::new(r"(?i)npm\s+install").unwrap(),
                Regex::new(r"(?i)\bmkdir\s+").unwrap(),
                Regex::new(r"(?i)configure").unwrap(),
            ],

            network_patterns: vec![
                Regex::new(r"(?i)\bcurl\s+").unwrap(),
                Regex::new(r"(?i)\bwget\s+").unwrap(),
                Regex::new(r"(?i)\bssh\s+").unwrap(),
                Regex::new(r"(?i)\bscp\s+").unwrap(),
                Regex::new(r"(?i)\bnetstat\s+|\bss\s+").unwrap(),
                Regex::new(r"(?i)\bping\s+").unwrap(),
                Regex::new(r"(?i)\bnc\s+|\bnetcat\s+").unwrap(),
                Regex::new(r"(?i)iptables|firewall").unwrap(),
            ],

            security_patterns: vec![
                Regex::new(r"(?i)\bchmod\s+").unwrap(),
                Regex::new(r"(?i)\bchown\s+").unwrap(),
                Regex::new(r"(?i)ssl|tls|certificate").unwrap(),
                Regex::new(r"(?i)\bgpg\s+|\bopenssl\s+").unwrap(),
                Regex::new(r"(?i)password|secret|credential").unwrap(),
                Regex::new(r"(?i)\bsudo\s+").unwrap(),
                Regex::new(r"(?i)encrypt|decrypt").unwrap(),
            ],
        }
    }

    /// Recognize all patterns in shell content.
    pub fn recognize(&self, content: &str) -> Vec<ShellPattern> {
        let mut patterns = Vec::new();

        // Check each pattern type
        if let Some(p) = self.check_deployment(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_error_handling(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_data_processing(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_automation(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_backup(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_monitoring(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_testing(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_build(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_cleanup(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_setup(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_network(content) {
            patterns.push(p);
        }
        if let Some(p) = self.check_security(content) {
            patterns.push(p);
        }

        // Sort by confidence
        patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // If no patterns found, return Unknown
        if patterns.is_empty() {
            patterns.push(ShellPattern::new(ShellPatternType::Unknown, 0.1));
        }

        patterns
    }

    /// Get the primary pattern (highest confidence).
    pub fn primary_pattern(&self, content: &str) -> ShellPattern {
        self.recognize(content)
            .into_iter()
            .next()
            .unwrap_or_else(|| ShellPattern::new(ShellPatternType::Unknown, 0.1))
    }

    /// Convert pattern to feature vector for clustering.
    pub fn pattern_to_features(&self, content: &str) -> Vec<f32> {
        let patterns = self.recognize(content);

        // 12-dimensional feature vector (one per pattern type)
        let mut features = vec![0.0f32; 12];

        for pattern in patterns {
            let idx = match pattern.pattern_type {
                ShellPatternType::Deployment => 0,
                ShellPatternType::ErrorHandling => 1,
                ShellPatternType::DataProcessing => 2,
                ShellPatternType::Automation => 3,
                ShellPatternType::Backup => 4,
                ShellPatternType::Monitoring => 5,
                ShellPatternType::Testing => 6,
                ShellPatternType::Build => 7,
                ShellPatternType::Cleanup => 8,
                ShellPatternType::Setup => 9,
                ShellPatternType::Network => 10,
                ShellPatternType::Security => 11,
                ShellPatternType::Unknown => continue,
            };
            features[idx] = features[idx].max(pattern.confidence);
        }

        features
    }

    // -------------------------------------------------------------------------
    // Pattern Checkers
    // -------------------------------------------------------------------------

    fn check_deployment(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Deployment, 0.0);

        for regex in &self.deployment_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        // Extract tools
        pattern.tools = self.extract_deployment_tools(content);
        if !pattern.tools.is_empty() {
            confidence += 0.1 * pattern.tools.len() as f32;
        }

        // Extract stages
        pattern.stages = self.extract_stages(content);

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_error_handling(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::ErrorHandling, 0.0);

        for regex in &self.error_patterns {
            if regex.is_match(content) {
                confidence += 0.25;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_data_processing(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::DataProcessing, 0.0);

        for regex in &self.data_patterns {
            if regex.is_match(content) {
                confidence += 0.15;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        // Extract tools
        let tools = [
            "awk", "sed", "jq", "grep", "cut", "sort", "uniq", "xargs", "tr",
        ];
        for tool in tools {
            if content.to_lowercase().contains(tool) {
                pattern.tools.push(tool.to_string());
            }
        }

        if confidence > 0.2 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_automation(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Automation, 0.0);

        for regex in &self.automation_patterns {
            if regex.is_match(content) {
                confidence += 0.25;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_backup(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Backup, 0.0);

        for regex in &self.backup_patterns {
            if regex.is_match(content) {
                confidence += 0.25;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_monitoring(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Monitoring, 0.0);

        for regex in &self.monitoring_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_testing(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Testing, 0.0);

        for regex in &self.testing_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_build(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Build, 0.0);

        for regex in &self.build_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        // Extract build tools
        let tools = [
            "make", "cmake", "cargo", "npm", "yarn", "go", "mvn", "gradle",
        ];
        for tool in tools {
            if content.to_lowercase().contains(tool) {
                pattern.tools.push(tool.to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_cleanup(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Cleanup, 0.0);

        for regex in &self.cleanup_patterns {
            if regex.is_match(content) {
                confidence += 0.25;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_setup(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Setup, 0.0);

        for regex in &self.setup_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_network(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Network, 0.0);

        for regex in &self.network_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    fn check_security(&self, content: &str) -> Option<ShellPattern> {
        let mut confidence: f32 = 0.0;
        let mut pattern = ShellPattern::new(ShellPatternType::Security, 0.0);

        for regex in &self.security_patterns {
            if regex.is_match(content) {
                confidence += 0.2;
                pattern.indicators.push(regex.as_str().to_string());
            }
        }

        if confidence > 0.3 {
            pattern.confidence = confidence.min(1.0);
            Some(pattern)
        } else {
            None
        }
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn extract_deployment_tools(&self, content: &str) -> Vec<String> {
        let mut tools = Vec::new();
        let content_lower = content.to_lowercase();

        let tool_patterns = [
            ("docker", "docker"),
            ("kubernetes", "kubectl"),
            ("helm", "helm"),
            ("terraform", "terraform"),
            ("ansible", "ansible"),
            ("aws", "aws"),
            ("gcloud", "gcloud"),
            ("azure", "az"),
            ("jenkins", "jenkins"),
            ("github-actions", "github"),
            ("gitlab-ci", "gitlab"),
        ];

        for (name, pattern) in tool_patterns {
            if content_lower.contains(pattern) {
                tools.push(name.to_string());
            }
        }

        tools
    }

    fn extract_stages(&self, content: &str) -> Vec<String> {
        let mut stages = Vec::new();
        let content_lower = content.to_lowercase();

        let stage_keywords = [
            ("build", &["build", "compile", "make"][..]),
            ("test", &["test", "pytest", "jest", "spec"]),
            ("deploy", &["deploy", "release", "rollout", "publish"]),
            ("verify", &["verify", "validate", "check", "health"]),
            ("cleanup", &["clean", "prune", "rm", "delete"]),
        ];

        for (stage_name, keywords) in stage_keywords {
            for keyword in keywords {
                if content_lower.contains(keyword) {
                    if !stages.contains(&stage_name.to_string()) {
                        stages.push(stage_name.to_string());
                    }
                    break;
                }
            }
        }

        stages
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognize_deployment() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            docker build -t myapp:latest .
            docker push myapp:latest
            kubectl apply -f k8s/deployment.yaml
            kubectl rollout status deployment/myapp
        "#;

        let patterns = recognizer.recognize(script);

        assert!(!patterns.is_empty());
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Deployment));

        let deploy = patterns
            .iter()
            .find(|p| p.pattern_type == ShellPatternType::Deployment)
            .unwrap();
        assert!(deploy.confidence > 0.5);
        assert!(deploy.tools.contains(&"docker".to_string()));
        assert!(deploy.tools.contains(&"kubernetes".to_string()));
    }

    #[test]
    fn test_recognize_error_handling() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            set -euo pipefail
            trap 'echo "Error occurred"; exit 1' ERR

            some_command || exit 1
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::ErrorHandling));

        let error = patterns
            .iter()
            .find(|p| p.pattern_type == ShellPatternType::ErrorHandling)
            .unwrap();
        assert!(error.confidence > 0.5);
    }

    #[test]
    fn test_recognize_data_processing() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            cat data.csv | awk -F',' '{print $1}' | sort | uniq -c | sort -rn
            cat config.json | jq '.settings.enabled'
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::DataProcessing));

        let data = patterns
            .iter()
            .find(|p| p.pattern_type == ShellPatternType::DataProcessing)
            .unwrap();
        assert!(data.tools.contains(&"awk".to_string()));
        assert!(data.tools.contains(&"jq".to_string()));
    }

    #[test]
    fn test_recognize_automation() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            # @daily
            # Runs every day at midnight

            while true; do
                check_health
                sleep 60
            done
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Automation));
    }

    #[test]
    fn test_recognize_backup() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            BACKUP_DIR=/backups/$(date +%Y%m%d)
            mkdir -p $BACKUP_DIR
            tar czf $BACKUP_DIR/data.tar.gz /var/data
            rsync -avz $BACKUP_DIR remote:/backups/
            pg_dump mydb > $BACKUP_DIR/db.sql
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Backup));
    }

    #[test]
    fn test_recognize_build() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            npm install
            npm run build
            cargo build --release
            make test
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Build));

        let build = patterns
            .iter()
            .find(|p| p.pattern_type == ShellPatternType::Build)
            .unwrap();
        assert!(build.tools.contains(&"npm".to_string()));
        assert!(build.tools.contains(&"cargo".to_string()));
        assert!(build.tools.contains(&"make".to_string()));
    }

    #[test]
    fn test_recognize_setup() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            apt-get update
            apt-get install -y build-essential
            pip install -r requirements.txt
            mkdir -p /var/app
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Setup));
    }

    #[test]
    fn test_recognize_network() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            curl -X POST https://api.example.com/data
            wget https://example.com/file.tar.gz
            ssh user@server 'ls -la'
            ping -c 4 google.com
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Network));
    }

    #[test]
    fn test_recognize_security() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            chmod 600 ~/.ssh/id_rsa
            chown root:root /etc/secret
            openssl enc -aes-256-cbc -salt -in file.txt -out file.enc
            sudo systemctl restart nginx
        "#;

        let patterns = recognizer.recognize(script);

        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == ShellPatternType::Security));
    }

    #[test]
    fn test_pattern_to_features() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            docker build -t app .
            kubectl apply -f deploy.yaml
        "#;

        let features = recognizer.pattern_to_features(script);

        assert_eq!(features.len(), 12);
        assert!(features[0] > 0.0); // Deployment feature
    }

    #[test]
    fn test_primary_pattern() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            docker build -t myapp .
            docker push myapp
            helm upgrade myapp ./chart
        "#;

        let primary = recognizer.primary_pattern(script);

        assert_eq!(primary.pattern_type, ShellPatternType::Deployment);
        assert!(primary.confidence > 0.5);
    }

    #[test]
    fn test_unknown_pattern() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            #!/bin/bash
            echo "Hello World"
        "#;

        let patterns = recognizer.recognize(script);

        // Should have at least one pattern (Unknown if nothing else)
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_multiple_patterns() {
        let recognizer = ShellPatternRecognizer::new();

        // Script with multiple patterns
        let script = r#"
            #!/bin/bash
            set -euo pipefail
            trap 'cleanup' EXIT

            docker build -t app .
            npm run test

            if [ $? -eq 0 ]; then
                docker push app
                kubectl apply -f deploy.yaml
            fi
        "#;

        let patterns = recognizer.recognize(script);

        // Should detect multiple patterns
        assert!(patterns.len() >= 2);

        let types: Vec<_> = patterns.iter().map(|p| p.pattern_type).collect();
        assert!(types.contains(&ShellPatternType::Deployment));
        assert!(types.contains(&ShellPatternType::ErrorHandling));
    }

    #[test]
    fn test_extract_stages() {
        let recognizer = ShellPatternRecognizer::new();

        let script = r#"
            docker build -t app .
            npm test
            docker push app
            kubectl apply -f deploy.yaml
            curl http://app/health
        "#;

        let patterns = recognizer.recognize(script);
        let deploy = patterns
            .iter()
            .find(|p| p.pattern_type == ShellPatternType::Deployment)
            .unwrap();

        assert!(deploy.stages.contains(&"build".to_string()));
        assert!(deploy.stages.contains(&"test".to_string()));
        assert!(deploy.stages.contains(&"deploy".to_string()));
    }
}
