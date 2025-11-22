#!/bin/bash
set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║            BackupForge Setup Script                        ║"
echo "║  Open-source backup and disaster recovery solution         ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}❌ Please do not run this script as root${NC}"
    exit 1
fi

# Check for Rust
echo -n "Checking for Rust... "
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(cargo --version | awk '{print $2}')
    echo -e "${GREEN}✓ Found Rust $RUST_VERSION${NC}"
else
    echo -e "${YELLOW}⚠ Rust not found${NC}"
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
    echo -e "${GREEN}✓ Rust installed${NC}"
fi

# Check for build dependencies
echo -n "Checking for build dependencies... "
if command -v pkg-config &> /dev/null; then
    echo -e "${GREEN}✓ Found${NC}"
else
    echo -e "${YELLOW}⚠ Missing build dependencies${NC}"
    echo "Please install: build-essential pkg-config libssl-dev"

    if command -v apt &> /dev/null; then
        echo "Run: sudo apt install build-essential pkg-config libssl-dev"
    elif command -v dnf &> /dev/null; then
        echo "Run: sudo dnf install gcc pkg-config openssl-devel"
    fi
    exit 1
fi

# Build the project
echo ""
echo "Building BackupForge..."
echo "This may take several minutes..."
cargo build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Create directories
echo ""
echo "Creating directories..."
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${CONFIG_DIR:-$HOME/.config/backupforge}"
DATA_DIR="${DATA_DIR:-$HOME/.local/share/backupforge}"

mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR/storage"

echo -e "${GREEN}✓ Directories created${NC}"
echo "  Install: $INSTALL_DIR"
echo "  Config:  $CONFIG_DIR"
echo "  Data:    $DATA_DIR"

# Install binary
echo ""
echo "Installing binary to $INSTALL_DIR..."
cp target/release/backupforge "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/backupforge"
echo -e "${GREEN}✓ Binary installed${NC}"

# Add to PATH if needed
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "${YELLOW}⚠ $INSTALL_DIR is not in your PATH${NC}"
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo ""
    echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
    echo ""
fi

# Create default config
CONFIG_FILE="$CONFIG_DIR/config.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    echo ""
    echo "Creating default configuration..."
    cat > "$CONFIG_FILE" << EOF
[storage]
type = "local"
path = "$DATA_DIR/storage"

[backup]
compression = "zstd"
compression_level = 3
encryption_enabled = true

[server]
bind_address = "127.0.0.1"
port = 8080
database_url = "sqlite://$DATA_DIR/db.sqlite"
jwt_secret = "$(openssl rand -base64 32)"
EOF
    echo -e "${GREEN}✓ Configuration created: $CONFIG_FILE${NC}"
else
    echo -e "${YELLOW}⚠ Configuration already exists: $CONFIG_FILE${NC}"
fi

# Initialize repository
echo ""
read -p "Initialize backup repository? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    "$INSTALL_DIR/backupforge" init --storage "$DATA_DIR/storage"
    echo -e "${GREEN}✓ Repository initialized${NC}"
fi

# Print success message
echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║            ✅ BackupForge Setup Complete!                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Installation directory: $INSTALL_DIR"
echo "Configuration file:     $CONFIG_FILE"
echo "Storage directory:      $DATA_DIR/storage"
echo ""
echo "Quick Start:"
echo "  1. Backup a directory:"
echo "     backupforge backup --source /path/to/data --storage $DATA_DIR/storage"
echo ""
echo "  2. List snapshots:"
echo "     backupforge list --storage $DATA_DIR/storage"
echo ""
echo "  3. Start the server:"
echo "     backupforge server --config $CONFIG_FILE"
echo ""
echo "  4. View help:"
echo "     backupforge --help"
echo ""
echo "Documentation: https://github.com/consigcody94/backupforge/docs"
echo ""
