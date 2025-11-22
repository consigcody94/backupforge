// API Base URL
const API_BASE = 'http://localhost:8080/api';

// State
let currentPage = 'dashboard';
let authToken = null;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    initNavigation();
    initModals();
    checkConnection();
    loadDashboard();

    // Auto-refresh data every 30 seconds
    setInterval(() => {
        if (currentPage === 'dashboard') {
            loadDashboard();
        }
    }, 30000);
});

// Navigation
function initNavigation() {
    const navItems = document.querySelectorAll('.nav-item');
    navItems.forEach(item => {
        item.addEventListener('click', (e) => {
            e.preventDefault();
            const page = item.dataset.page;
            navigateTo(page);
        });
    });
}

function navigateTo(page) {
    // Update active nav item
    document.querySelectorAll('.nav-item').forEach(item => {
        item.classList.toggle('active', item.dataset.page === page);
    });

    // Show page
    document.querySelectorAll('.page').forEach(p => {
        p.classList.toggle('active', p.id === `${page}-page`);
    });

    // Update title
    const titles = {
        dashboard: 'Dashboard',
        snapshots: 'Snapshots',
        jobs: 'Backup Jobs',
        storage: 'Storage',
        settings: 'Settings'
    };
    document.getElementById('page-title').textContent = titles[page] || page;

    currentPage = page;

    // Load page data
    switch (page) {
        case 'dashboard':
            loadDashboard();
            break;
        case 'snapshots':
            loadSnapshots();
            break;
        case 'jobs':
            loadJobs();
            break;
        case 'storage':
            loadStorage();
            break;
    }
}

// Check API Connection
async function checkConnection() {
    const statusEl = document.getElementById('connection-status');
    try {
        const response = await fetch(`${API_BASE.replace('/api', '')}/health`);
        if (response.ok) {
            statusEl.textContent = 'Connected';
            statusEl.parentElement.querySelector('.status-dot').style.background = '#10b981';
        } else {
            throw new Error('API not responding');
        }
    } catch (error) {
        statusEl.textContent = 'Disconnected';
        statusEl.parentElement.querySelector('.status-dot').style.background = '#ef4444';
        console.error('Connection error:', error);
    }
}

// Load Dashboard Data
async function loadDashboard() {
    try {
        const stats = await fetchAPI('/storage/stats');
        updateDashboardStats(stats);
    } catch (error) {
        console.error('Failed to load dashboard:', error);
        // Show demo data
        updateDashboardStats({
            total_bytes: 50000000000, // 50 GB
            total_chunks: 125000,
            compressed_bytes: 25000000000 // 25 GB
        });
    }
}

function updateDashboardStats(stats) {
    const totalGB = (stats.total_bytes / 1073741824).toFixed(2);
    const compressedGB = (stats.compressed_bytes / 1073741824).toFixed(2);
    const dedupRatio = ((1 - stats.compressed_bytes / stats.total_bytes) * 100).toFixed(1);

    document.getElementById('total-storage').textContent = `${totalGB} GB`;
    document.getElementById('dedup-ratio').textContent = `${dedupRatio}%`;

    // Update storage chart
    const usedPercent = 50; // Demo data
    const circumference = 2 * Math.PI * 80;
    const offset = circumference - (usedPercent / 100) * circumference;
    document.getElementById('storage-progress').style.strokeDashoffset = offset;
    document.getElementById('storage-percent').textContent = `${usedPercent}%`;
    document.getElementById('storage-used').textContent = `${totalGB} GB`;
    document.getElementById('storage-total').textContent = '100 GB';
    document.getElementById('storage-available').textContent = `${(100 - totalGB).toFixed(2)} GB`;
}

// Load Snapshots
async function loadSnapshots() {
    const tbody = document.getElementById('snapshots-table');

    try {
        const snapshots = await fetchAPI('/snapshots');
        if (snapshots && snapshots.length > 0) {
            tbody.innerHTML = snapshots.map(snapshot => `
                <tr>
                    <td>${snapshot.name}</td>
                    <td>${formatDate(snapshot.created_at)}</td>
                    <td>${formatBytes(snapshot.total_size)}</td>
                    <td>${snapshot.file_count}</td>
                    <td><span class="badge badge-success">Complete</span></td>
                    <td>
                        <button class="btn btn-sm">Restore</button>
                        <button class="btn btn-sm">Delete</button>
                    </td>
                </tr>
            `).join('');
        } else {
            tbody.innerHTML = `
                <tr class="empty-row">
                    <td colspan="6">
                        <div class="empty-state">
                            <div class="empty-icon">📸</div>
                            <p>No snapshots available</p>
                        </div>
                    </td>
                </tr>
            `;
        }
    } catch (error) {
        console.error('Failed to load snapshots:', error);
    }
}

// Load Jobs
async function loadJobs() {
    const container = document.getElementById('jobs-list');

    try {
        const jobs = await fetchAPI('/jobs');
        if (jobs && jobs.length > 0) {
            container.innerHTML = jobs.map(job => `
                <div class="job-card">
                    <h4>${job.name}</h4>
                    <p>${job.source}</p>
                    <p>Schedule: ${job.schedule || 'Manual'}</p>
                    <button class="btn btn-sm">Run Now</button>
                </div>
            `).join('');
        } else {
            container.innerHTML = `
                <div class="empty-state">
                    <div class="empty-icon">⚙️</div>
                    <p>No backup jobs configured</p>
                    <button class="btn btn-primary">Create First Job</button>
                </div>
            `;
        }
    } catch (error) {
        console.error('Failed to load jobs:', error);
    }
}

// Load Storage Stats
async function loadStorage() {
    try {
        const stats = await fetchAPI('/storage/stats');
        document.getElementById('stat-chunks').textContent = stats.total_chunks || 0;
        document.getElementById('stat-bytes').textContent = formatBytes(stats.total_bytes || 0);
        document.getElementById('stat-compressed').textContent = formatBytes(stats.compressed_bytes || 0);

        const savings = stats.total_bytes > 0
            ? ((1 - stats.compressed_bytes / stats.total_bytes) * 100).toFixed(1)
            : 0;
        document.getElementById('stat-savings').textContent = `${savings}%`;
    } catch (error) {
        console.error('Failed to load storage stats:', error);
    }
}

// Modal handling
function initModals() {
    const newBackupBtn = document.getElementById('new-backup-btn');
    const modal = document.getElementById('new-backup-modal');
    const closeButtons = modal.querySelectorAll('.modal-close');
    const createBtn = document.getElementById('create-backup-btn');

    newBackupBtn.addEventListener('click', () => {
        modal.classList.add('active');
    });

    closeButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            modal.classList.remove('active');
        });
    });

    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            modal.classList.remove('active');
        }
    });

    createBtn.addEventListener('click', createBackup);
}

async function createBackup() {
    const source = document.getElementById('backup-source').value;
    const name = document.getElementById('backup-name').value;
    const encryption = document.getElementById('backup-encryption').checked;
    const compression = parseInt(document.getElementById('backup-compression').value);

    if (!source) {
        alert('Please enter a source path');
        return;
    }

    try {
        const job = {
            name: name || `Backup ${new Date().toISOString()}`,
            source: {
                type: 'LocalPath',
                path: source,
                excludes: []
            },
            destination: '/var/lib/backupforge/storage',
            schedule: null,
            retention_days: 30,
            enabled: true,
            encryption_enabled: encryption,
            compression_level: compression
        };

        const result = await fetchAPI('/jobs', {
            method: 'POST',
            body: JSON.stringify(job)
        });

        alert('Backup job created successfully!');
        document.getElementById('new-backup-modal').classList.remove('active');
        loadDashboard();
    } catch (error) {
        alert('Failed to create backup: ' + error.message);
    }
}

// API Helper
async function fetchAPI(endpoint, options = {}) {
    const url = `${API_BASE}${endpoint}`;
    const headers = {
        'Content-Type': 'application/json',
        ...options.headers
    };

    if (authToken) {
        headers['Authorization'] = `Bearer ${authToken}`;
    }

    const response = await fetch(url, {
        ...options,
        headers
    });

    if (!response.ok) {
        throw new Error(`API error: ${response.statusText}`);
    }

    return response.json();
}

// Utility Functions
function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDate(date) {
    return new Date(date).toLocaleString();
}
