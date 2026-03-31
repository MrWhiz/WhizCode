use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringFiles {
    pub product: ProductSteering,
    pub tech: TechSteering,
    pub structure: StructureSteering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSteering {
    pub purpose: String,
    pub target_users: Vec<String>,
    pub key_features: Vec<String>,
    pub business_objectives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechSteering {
    pub frameworks: Vec<String>,
    pub libraries: Vec<String>,
    pub development_tools: Vec<String>,
    pub technical_constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureSteering {
    pub file_organization: String,
    pub naming_conventions: String,
    pub import_patterns: String,
    pub architectural_decisions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub file: String,
    pub field: String,
    pub message: String,
}

pub struct SteeringFileManager;

impl SteeringFileManager {
    /// Load steering files from workspace
    pub fn load_steering_files(workspace_path: &str) -> Result<SteeringFiles, String> {
        let steering_dir = Path::new(workspace_path).join(".kiro/steering");

        let product = Self::load_product_steering(&steering_dir)?;
        let tech = Self::load_tech_steering(&steering_dir)?;
        let structure = Self::load_structure_steering(&steering_dir)?;

        Ok(SteeringFiles {
            product,
            tech,
            structure,
        })
    }

    /// Load product.md steering file
    fn load_product_steering(steering_dir: &Path) -> Result<ProductSteering, String> {
        let product_path = steering_dir.join("product.md");

        if !product_path.exists() {
            return Ok(ProductSteering {
                purpose: String::new(),
                target_users: vec![],
                key_features: vec![],
                business_objectives: vec![],
            });
        }

        let content = fs::read_to_string(&product_path)
            .map_err(|e| format!("Failed to read product.md: {}", e))?;

        Self::parse_product_steering(&content)
    }

    /// Load tech.md steering file
    fn load_tech_steering(steering_dir: &Path) -> Result<TechSteering, String> {
        let tech_path = steering_dir.join("tech.md");

        if !tech_path.exists() {
            return Ok(TechSteering {
                frameworks: vec![],
                libraries: vec![],
                development_tools: vec![],
                technical_constraints: vec![],
            });
        }

        let content = fs::read_to_string(&tech_path)
            .map_err(|e| format!("Failed to read tech.md: {}", e))?;

        Self::parse_tech_steering(&content)
    }

    /// Load structure.md steering file
    fn load_structure_steering(steering_dir: &Path) -> Result<StructureSteering, String> {
        let structure_path = steering_dir.join("structure.md");

        if !structure_path.exists() {
            return Ok(StructureSteering {
                file_organization: String::new(),
                naming_conventions: String::new(),
                import_patterns: String::new(),
                architectural_decisions: vec![],
            });
        }

        let content = fs::read_to_string(&structure_path)
            .map_err(|e| format!("Failed to read structure.md: {}", e))?;

        Self::parse_structure_steering(&content)
    }

    /// Parse product.md content
    fn parse_product_steering(content: &str) -> Result<ProductSteering, String> {
        let mut purpose = String::new();
        let mut target_users = vec![];
        let mut key_features = vec![];
        let mut business_objectives = vec![];

        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = "";

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("## Purpose") {
                current_section = "purpose";
            } else if trimmed.starts_with("## Target Users") {
                current_section = "target_users";
            } else if trimmed.starts_with("## Key Features") {
                current_section = "key_features";
            } else if trimmed.starts_with("## Business Objectives") {
                current_section = "business_objectives";
            } else if trimmed.starts_with("- ") {
                let item = trimmed.strip_prefix("- ").unwrap_or("").to_string();
                match current_section {
                    "target_users" => target_users.push(item),
                    "key_features" => key_features.push(item),
                    "business_objectives" => business_objectives.push(item),
                    _ => {}
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("#") && current_section == "purpose" {
                if !purpose.is_empty() {
                    purpose.push(' ');
                }
                purpose.push_str(trimmed);
            }
        }

        Ok(ProductSteering {
            purpose,
            target_users,
            key_features,
            business_objectives,
        })
    }

    /// Parse tech.md content
    fn parse_tech_steering(content: &str) -> Result<TechSteering, String> {
        let mut frameworks = vec![];
        let mut libraries = vec![];
        let mut development_tools = vec![];
        let mut technical_constraints = vec![];

        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = "";

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("## Frameworks") {
                current_section = "frameworks";
            } else if trimmed.starts_with("## Libraries") {
                current_section = "libraries";
            } else if trimmed.starts_with("## Development Tools") {
                current_section = "development_tools";
            } else if trimmed.starts_with("## Technical Constraints") {
                current_section = "technical_constraints";
            } else if trimmed.starts_with("- ") {
                let item = trimmed.strip_prefix("- ").unwrap_or("").to_string();
                match current_section {
                    "frameworks" => frameworks.push(item),
                    "libraries" => libraries.push(item),
                    "development_tools" => development_tools.push(item),
                    "technical_constraints" => technical_constraints.push(item),
                    _ => {}
                }
            }
        }

        Ok(TechSteering {
            frameworks,
            libraries,
            development_tools,
            technical_constraints,
        })
    }

    /// Parse structure.md content
    fn parse_structure_steering(content: &str) -> Result<StructureSteering, String> {
        let mut file_organization = String::new();
        let mut naming_conventions = String::new();
        let mut import_patterns = String::new();
        let mut architectural_decisions = vec![];

        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = "";

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("## File Organization") {
                current_section = "file_organization";
            } else if trimmed.starts_with("## Naming Conventions") {
                current_section = "naming_conventions";
            } else if trimmed.starts_with("## Import Patterns") {
                current_section = "import_patterns";
            } else if trimmed.starts_with("## Architectural Decisions") {
                current_section = "architectural_decisions";
            } else if trimmed.starts_with("- ") {
                let item = trimmed.strip_prefix("- ").unwrap_or("").to_string();
                if current_section == "architectural_decisions" {
                    architectural_decisions.push(item);
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("#") {
                match current_section {
                    "file_organization" => {
                        if !file_organization.is_empty() {
                            file_organization.push('\n');
                        }
                        file_organization.push_str(trimmed);
                    }
                    "naming_conventions" => {
                        if !naming_conventions.is_empty() {
                            naming_conventions.push('\n');
                        }
                        naming_conventions.push_str(trimmed);
                    }
                    "import_patterns" => {
                        if !import_patterns.is_empty() {
                            import_patterns.push('\n');
                        }
                        import_patterns.push_str(trimmed);
                    }
                    _ => {}
                }
            }
        }

        Ok(StructureSteering {
            file_organization,
            naming_conventions,
            import_patterns,
            architectural_decisions,
        })
    }

    /// Validate steering files
    pub fn validate_steering_files(steering: &SteeringFiles) -> Vec<ValidationError> {
        let mut errors = vec![];

        // Validate product steering
        if steering.product.purpose.is_empty() {
            errors.push(ValidationError {
                file: "product.md".to_string(),
                field: "purpose".to_string(),
                message: "Purpose cannot be empty".to_string(),
            });
        }

        if steering.product.target_users.is_empty() {
            errors.push(ValidationError {
                file: "product.md".to_string(),
                field: "target_users".to_string(),
                message: "At least one target user should be defined".to_string(),
            });
        }

        if steering.product.key_features.is_empty() {
            errors.push(ValidationError {
                file: "product.md".to_string(),
                field: "key_features".to_string(),
                message: "At least one key feature should be defined".to_string(),
            });
        }

        // Validate tech steering
        if steering.tech.frameworks.is_empty() {
            errors.push(ValidationError {
                file: "tech.md".to_string(),
                field: "frameworks".to_string(),
                message: "At least one framework should be defined".to_string(),
            });
        }

        if steering.tech.libraries.is_empty() {
            errors.push(ValidationError {
                file: "tech.md".to_string(),
                field: "libraries".to_string(),
                message: "At least one library should be defined".to_string(),
            });
        }

        // Validate structure steering
        if steering.structure.file_organization.is_empty() {
            errors.push(ValidationError {
                file: "structure.md".to_string(),
                field: "file_organization".to_string(),
                message: "File organization should be defined".to_string(),
            });
        }

        if steering.structure.naming_conventions.is_empty() {
            errors.push(ValidationError {
                file: "structure.md".to_string(),
                field: "naming_conventions".to_string(),
                message: "Naming conventions should be defined".to_string(),
            });
        }

        errors
    }

    /// Create default steering files
    pub fn create_default_steering_files(workspace_path: &str) -> Result<(), String> {
        let steering_dir = Path::new(workspace_path).join(".kiro/steering");
        fs::create_dir_all(&steering_dir)
            .map_err(|e| format!("Failed to create steering directory: {}", e))?;

        // Create product.md
        let product_content = r#"# Product Steering

## Purpose
Define the purpose and goals of this project.

## Target Users
- Primary users
- Secondary users

## Key Features
- Feature 1
- Feature 2

## Business Objectives
- Objective 1
- Objective 2
"#;

        fs::write(steering_dir.join("product.md"), product_content)
            .map_err(|e| format!("Failed to create product.md: {}", e))?;

        // Create tech.md
        let tech_content = r#"# Tech Steering

## Frameworks
- React
- Tauri

## Libraries
- Serde
- Tokio

## Development Tools
- Rust
- Node.js

## Technical Constraints
- Constraint 1
- Constraint 2
"#;

        fs::write(steering_dir.join("tech.md"), tech_content)
            .map_err(|e| format!("Failed to create tech.md: {}", e))?;

        // Create structure.md
        let structure_content = r#"# Structure Steering

## File Organization
Organize files by feature/domain, not by type.

## Naming Conventions
Use camelCase for variables and functions, PascalCase for components and classes.

## Import Patterns
Use absolute imports from src/ directory.

## Architectural Decisions
- Decision 1
- Decision 2
"#;

        fs::write(steering_dir.join("structure.md"), structure_content)
            .map_err(|e| format!("Failed to create structure.md: {}", e))?;

        Ok(())
    }

    /// Get steering context for agent reasoning
    pub fn get_steering_context(steering: &SteeringFiles) -> String {
        let mut context = String::new();

        context.push_str("## Project Steering Context\n\n");

        context.push_str("### Product\n");
        context.push_str(&format!("**Purpose**: {}\n\n", steering.product.purpose));
        context.push_str("**Target Users**: ");
        context.push_str(&steering.product.target_users.join(", "));
        context.push_str("\n\n");
        context.push_str("**Key Features**: ");
        context.push_str(&steering.product.key_features.join(", "));
        context.push_str("\n\n");

        context.push_str("### Technology\n");
        context.push_str("**Frameworks**: ");
        context.push_str(&steering.tech.frameworks.join(", "));
        context.push_str("\n\n");
        context.push_str("**Libraries**: ");
        context.push_str(&steering.tech.libraries.join(", "));
        context.push_str("\n\n");

        context.push_str("### Structure\n");
        context.push_str(&format!("**File Organization**: {}\n\n", steering.structure.file_organization));
        context.push_str(&format!("**Naming Conventions**: {}\n\n", steering.structure.naming_conventions));

        context
    }
}
