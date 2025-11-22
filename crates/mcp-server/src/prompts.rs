use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub fn get_prompts_list() -> Vec<Value> {
    vec![
        json!({
            "name": "create_backup_plan",
            "description": "Help create a comprehensive backup plan for a system",
            "arguments": [
                {
                    "name": "system_type",
                    "description": "Type of system (personal, small_business, enterprise)",
                    "required": true
                }
            ]
        }),
        json!({
            "name": "disaster_recovery",
            "description": "Guide through disaster recovery process",
            "arguments": [
                {
                    "name": "scenario",
                    "description": "Type of disaster (data_loss, ransomware, hardware_failure)",
                    "required": true
                }
            ]
        }),
        json!({
            "name": "optimize_storage",
            "description": "Analyze and suggest storage optimizations",
            "arguments": []
        }),
    ]
}

pub fn get_prompt(prompt_name: &str, arguments: &Value) -> Result<Vec<Value>> {
    match prompt_name {
        "create_backup_plan" => create_backup_plan_prompt(arguments),
        "disaster_recovery" => disaster_recovery_prompt(arguments),
        "optimize_storage" => optimize_storage_prompt(),
        _ => Err(anyhow!("Unknown prompt: {}", prompt_name)),
    }
}

fn create_backup_plan_prompt(arguments: &Value) -> Result<Vec<Value>> {
    let system_type = arguments["system_type"]
        .as_str()
        .unwrap_or("personal");

    let message = match system_type {
        "personal" => {
            "I'll help you create a backup plan for your personal system.\n\n\
            Let's consider:\n\
            1. What data is most important to you? (Documents, Photos, Videos, etc.)\n\
            2. How often does this data change?\n\
            3. Where should backups be stored? (Local external drive, Cloud storage, or both)\n\
            4. How long should you keep backups?\n\
            5. Do you need encryption for sensitive data?\n\n\
            Based on your answers, I'll recommend:\n\
            - Backup frequency (daily, weekly, etc.)\n\
            - Storage strategy (3-2-1 rule: 3 copies, 2 different media, 1 offsite)\n\
            - Retention policy\n\
            - Automation setup using BackupForge"
        }
        "small_business" => {
            "I'll help you create a backup plan for your small business.\n\n\
            Let's assess:\n\
            1. Critical business data (databases, documents, customer data)\n\
            2. Server configurations and applications\n\
            3. Compliance requirements (GDPR, HIPAA, etc.)\n\
            4. Recovery Time Objective (RTO) - how quickly you need to recover\n\
            5. Recovery Point Objective (RPO) - how much data loss is acceptable\n\n\
            I'll recommend:\n\
            - Backup schedule for different data types\n\
            - Multi-tier storage strategy\n\
            - Encryption and security measures\n\
            - Testing and verification procedures\n\
            - BackupForge job configurations"
        }
        "enterprise" => {
            "I'll help you create an enterprise-grade backup plan.\n\n\
            We'll cover:\n\
            1. Data classification and prioritization\n\
            2. Multi-site backup strategy\n\
            3. Disaster recovery planning\n\
            4. High-availability requirements\n\
            5. Compliance and audit requirements\n\
            6. VM and container backup strategies\n\
            7. Database backup and point-in-time recovery\n\n\
            I'll provide:\n\
            - Comprehensive backup architecture\n\
            - Multi-tenant setup for different departments\n\
            - Automated testing and verification\n\
            - Documentation and runbooks\n\
            - BackupForge deployment strategy"
        }
        _ => "Please specify system_type: personal, small_business, or enterprise"
    };

    Ok(vec![
        json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!("Help me create a backup plan for a {} system.", system_type)
            }
        }),
        json!({
            "role": "assistant",
            "content": {
                "type": "text",
                "text": message
            }
        }),
    ])
}

fn disaster_recovery_prompt(arguments: &Value) -> Result<Vec<Value>> {
    let scenario = arguments["scenario"].as_str().unwrap_or("data_loss");

    let message = match scenario {
        "data_loss" => {
            "I'll guide you through data loss recovery.\n\n\
            Step 1: Assess the situation\n\
            - What data was lost?\n\
            - When did it happen?\n\
            - What was the cause?\n\n\
            Step 2: Check available backups\n\
            Use: `backupforge list --storage /your/storage`\n\n\
            Step 3: Identify the right snapshot\n\
            - Find a snapshot from before the data loss\n\
            - Verify it contains the needed data\n\n\
            Step 4: Restore the data\n\
            Use: `backupforge restore --snapshot <id> --target /recovery/path`\n\n\
            Step 5: Verify restored data\n\
            - Check file integrity\n\
            - Verify completeness\n\n\
            Step 6: Prevent future incidents\n\
            - Increase backup frequency\n\
            - Add monitoring alerts\n\
            - Review access controls"
        }
        "ransomware" => {
            "I'll guide you through ransomware recovery.\n\n\
            ⚠️ CRITICAL: DO NOT PAY THE RANSOM\n\n\
            Step 1: Isolate affected systems\n\
            - Disconnect from network immediately\n\
            - Power off infected machines\n\
            - Identify infection scope\n\n\
            Step 2: Preserve evidence\n\
            - Document everything\n\
            - Keep encrypted files (for forensics)\n\n\
            Step 3: Identify clean backups\n\
            - Find backups from before infection\n\
            - Verify backups are clean\n\
            - Use immutable backups if available\n\n\
            Step 4: Clean and restore\n\
            - Wipe affected systems\n\
            - Reinstall OS and software\n\
            - Restore from verified clean backups\n\n\
            Step 5: Security hardening\n\
            - Update all software\n\
            - Review and strengthen security\n\
            - Enable backup encryption\n\
            - Implement 3-2-1 backup strategy\n\n\
            Step 6: Monitor and test\n\
            - Set up monitoring\n\
            - Regular backup testing\n\
            - Security awareness training"
        }
        "hardware_failure" => {
            "I'll guide you through hardware failure recovery.\n\n\
            Step 1: Assess the failure\n\
            - What hardware failed? (disk, server, etc.)\n\
            - Is data recoverable from failed hardware?\n\
            - What's the business impact?\n\n\
            Step 2: Acquire replacement hardware\n\
            - Same or compatible hardware\n\
            - Verify specifications\n\n\
            Step 3: Prepare for restoration\n\
            - Install OS on new hardware\n\
            - Install BackupForge\n\
            - Configure storage access\n\n\
            Step 4: Restore from backups\n\
            Use: `backupforge restore --snapshot <latest> --target /new/location`\n\n\
            Step 5: Verify and test\n\
            - Check all restored data\n\
            - Test application functionality\n\
            - Verify integrations\n\n\
            Step 6: Resume operations\n\
            - Bring system online\n\
            - Monitor closely\n\
            - Update documentation\n\n\
            Future prevention:\n\
            - Use RAID for redundancy\n\
            - Monitor hardware health (SMART)\n\
            - Maintain spare hardware\n\
            - Regular backup testing"
        }
        _ => "Please specify scenario: data_loss, ransomware, or hardware_failure"
    };

    Ok(vec![
        json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!("Help me recover from {}.", scenario)
            }
        }),
        json!({
            "role": "assistant",
            "content": {
                "type": "text",
                "text": message
            }
        }),
    ])
}

fn optimize_storage_prompt() -> Result<Vec<Value>> {
    Ok(vec![
        json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": "Help me optimize my backup storage."
            }
        }),
        json!({
            "role": "assistant",
            "content": {
                "type": "text",
                "text": "I'll help you optimize your BackupForge storage.\n\n\
                Let's analyze current usage:\n\
                1. Use `backupforge stats` to see current storage\n\
                2. Check deduplication ratio\n\
                3. Review snapshot retention policy\n\n\
                Optimization strategies:\n\n\
                1. Deduplication\n\
                - Already enabled by default\n\
                - Using content-defined chunking (CDC)\n\
                - Typical savings: 40-60% for similar data\n\n\
                2. Compression\n\
                - Default: Zstd level 3 (balanced)\n\
                - For speed: Use LZ4\n\
                - For max compression: Use Zstd level 15-22\n\
                - Configure in backup job or config file\n\n\
                3. Retention Policy\n\
                - Keep recent: 7-30 daily backups\n\
                - Keep monthly: 12 months\n\
                - Keep yearly: 3-7 years\n\
                - Delete old snapshots regularly\n\n\
                4. Exclude unnecessary data\n\
                - Cache directories (.cache/)\n\
                - Temporary files (*.tmp, *.temp)\n\
                - Build artifacts (node_modules/, target/)\n\
                - System files that can be recreated\n\n\
                5. Storage tiering\n\
                - Recent backups: Fast local storage\n\
                - Older backups: S3 or cold storage\n\
                - Use lifecycle policies\n\n\
                6. Regular cleanup\n\
                - Run garbage collection\n\
                - Remove orphaned chunks\n\
                - Verify backup integrity\n\n\
                Monitoring:\n\
                - Track storage growth\n\
                - Monitor dedup ratio\n\
                - Set up alerts for quota limits\n\n\
                Want me to analyze your specific setup?"
            }
        }),
    ])
}
