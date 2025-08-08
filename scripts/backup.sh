#!/bin/bash
#
# Git Repository Backup Script - v2.2 (Definitive Fix)
# Creates bundle backups for a defined list of repositories within the project.
#

set -e
set -o pipefail

# --- Configuration ---
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." &> /dev/null && pwd )"
BACKUP_DATE=$(date +%Y%m%d_%H%M%S)

REPOS_TO_BACKUP=(
    "$PROJECT_ROOT"
)

BACKUP_LOCATIONS=(
    "$HOME/backups"
    "$HOME/icloud/Alan/backups/"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# --- Helper Functions ---
log_info() { echo -e "${BLUE}[INFO]${NC} $1" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# DEFINITIVE FIX: Restored the missing helper function.
create_backup_dirs() {
    for location in "${BACKUP_LOCATIONS[@]}"; do
        if [ ! -d "$location" ]; then
            log_info "Creating backup directory: $location"
            mkdir -p "$location" || log_warn "Failed to create $location"
        fi
    done
}

create_bundle() {
    local repo_dir="$1"
    local repo_basename=$(basename $(dirname "$repo_dir"))_$(basename "$repo_dir")
    local bundle_name_prefix=$repo_basename

    log_info "Creating bundle for '$repo_basename' repository..."
    
    if ! git -C "$repo_dir" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        log_error "'$repo_dir' is not a git repository."
        return 1
    fi

    local bundle_file="${PROJECT_ROOT}/${bundle_name_prefix}_backup_${BACKUP_DATE}.bundle"

    if git -C "$repo_dir" bundle create "$bundle_file" --all >/dev/null 2>&1; then
        log_success "Bundle created: $bundle_file"
        echo "$bundle_file"
    else
        log_error "Failed to create bundle for '$repo_basename'"
        return 1
    fi
}

verify_bundle() {
    local bundle_file="$1"
    log_info "Verifying bundle: $(basename "$bundle_file")"
    if [ ! -s "$bundle_file" ]; then
        log_error "Bundle file is empty or does not exist: $bundle_file"
        return 1
    fi
    if git bundle verify "$bundle_file" >/dev/null 2>&1; then
        log_success "Bundle verification passed."
    else
        log_error "Bundle verification failed for $(basename "$bundle_file")"
        return 1
    fi
}

copy_to_backup_locations() {
    local bundle_file="$1"
    log_info "Copying $(basename "$bundle_file") to backup locations..."
    for location in "${BACKUP_LOCATIONS[@]}"; do
        if [ -d "$location" ]; then
            cp "$bundle_file" "$location/" && log_success "Copied to: $location" || log_warn "Failed to copy to: $location"
        else
            log_warn "Backup location not accessible: $location"
        fi
    done
}

cleanup_old_bundles() {
    local bundle_prefix="$1"
    log_info "Cleaning up old '$bundle_prefix' bundles in project root..."
    ls -t "${PROJECT_ROOT}/${bundle_prefix}_backup_"*.bundle 2>/dev/null | tail -n +6 | xargs -r rm -f
}

# --- Main Script ---
main() {
    log_info "Starting Dual-Repository Backup Process"
    create_backup_dirs
    
    local created_bundles=()

    for repo_dir in "${REPOS_TO_BACKUP[@]}"; do
        echo "--------------------------------------" >&2
        log_info "Processing repository: $repo_dir"
        
        if [ ! -d "$repo_dir/.git" ]; then
            log_warn "Directory '$repo_dir' is not a git repository. Skipping."
            continue
        fi

        bundle_path=$(create_bundle "$repo_dir")
        
        if [ -n "$bundle_path" ]; then
            if verify_bundle "$bundle_path"; then
                copy_to_backup_locations "$bundle_path"
                created_bundles+=("$bundle_path")
                bundle_prefix=$(basename "$bundle_path" | sed -E "s/_backup_.*.bundle//")
                cleanup_old_bundles "$bundle_prefix"
            fi
        else
            log_error "Bundle creation failed for '$repo_dir'"
        fi
    done
    
    echo "--------------------------------------" >&2
    log_success "Backup process completed!"
    echo >&2
    log_info "SUMMARY:"
    if [ ${#created_bundles[@]} -gt 0 ]; then
        for bundle in "${created_bundles[@]}"; do
            log_success "Successfully created: $(basename "$bundle")"
        done
    else
        log_error "No bundles were created."
    fi
}

main "$@"
