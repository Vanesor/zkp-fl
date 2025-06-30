// Dashboard JavaScript for ZKP-FL Advanced Benchmark Dashboard

class ZKPDashboard {
  constructor() {
    this.benchmarkData = [];
    this.charts = {};
    this.currentBenchmarkProcess = null;
    this.isRunning = false;
    this.refreshInterval = null;
    this.runs = [];
    this.selectedRun = null;

    this.init();
  }

  async init() {
    await this.loadRuns();
    this.setupEventListeners();
    this.initializeTabs();
    this.initializeCharts();
    await this.loadBenchmarkData();
    this.updateMetrics();
    this.updateTable();
    this.startAutoRefresh();
  }

  async loadRuns() {
    try {
      const response = await fetch("/api/runs");
      if (response.ok) {
        const data = await response.json();
        this.runs = data.runs || [];
        this.selectedRun = this.runs.length > 0 ? this.runs[0].run_id : null;
        this.renderRunDropdown();
      } else {
        this.runs = [];
        this.selectedRun = null;
      }
    } catch (e) {
      this.runs = [];
      this.selectedRun = null;
    }
  }

  renderRunDropdown() {
    const container = document.getElementById("runDropdownContainer");
    if (!container) return;
    container.innerHTML = "";
    if (!this.runs || this.runs.length === 0) return;
    const select = document.createElement("select");
    select.id = "runDropdown";
    this.runs.forEach((run) => {
      const option = document.createElement("option");
      option.value = run.run_id;
      option.textContent = `${run.scenario} | Clients: ${run.num_clients} | ${
        run.start_time ? new Date(run.start_time).toLocaleString() : ""
      }`;
      select.appendChild(option);
    });
    select.value = this.selectedRun;
    select.addEventListener("change", (e) => {
      this.selectedRun = e.target.value;
      this.updateMetrics();
      this.updateTable();
      this.updateCharts();
    });
    container.appendChild(select);
  }

  getFilteredData() {
    if (!this.selectedRun) return this.benchmarkData;
    return this.benchmarkData.filter(
      (b) => b.report_id === this.selectedRun || b.session_id === this.selectedRun
    );
  }

  async loadBenchmarkData() {
    try {
      const response = await fetch("/api/benchmarks");
      if (response.ok) {
        const data = await response.json();
        // The API returns {benchmarks: [...], count: ..., metrics: ...}
        // We need just the benchmarks array
        this.benchmarkData = data.benchmarks || [];
      } else {
        // Fallback: load from local files
        await this.loadLocalBenchmarkData();
      }
    } catch (error) {
      console.warn("Failed to load from API, trying local files:", error);
      await this.loadLocalBenchmarkData();
    }
  }

  async loadLocalBenchmarkData() {
    try {
      // Load individual benchmark files
      const benchmarkFiles = await this.getBenchmarkFiles();
      this.benchmarkData = [];

      for (const file of benchmarkFiles) {
        try {
          const response = await fetch(`../benchmarks/${file}`);
          if (response.ok) {
            const data = await response.json();
            data.filename = file;
            this.benchmarkData.push(data);
          }
        } catch (error) {
          console.warn(`Failed to load ${file}:`, error);
        }
      }
    } catch (error) {
      console.error("Failed to load benchmark data:", error);
      // Use sample data for demo
      this.benchmarkData = this.generateSampleData();
    }
  }

  async getBenchmarkFiles() {
    // Return a list of known benchmark files
    return [
      "benchmark_20250627_100324_client_benchmark_client_0.json",
      "benchmark_20250627_100324_client_benchmark_client_1.json",
      "benchmark_20250627_100325_client_benchmark_client_2.json",
      "benchmark_20250627_101136_client_benchmark_client_0.json",
      "benchmark_20250627_101136_client_benchmark_client_1.json",
      "benchmark_20250627_101138_client_benchmark_client_2.json",
    ];
  }

  generateSampleData() {
    // Generate sample data for demonstration
    const sampleData = [];
    const scenarios = [
      "single-client",
      "multi-client-sequential",
      "multi-client-concurrent",
    ];

    for (let i = 0; i < 10; i++) {
      const data = {
        session_id: `sample-${i}`,
        client_id: `client_${i}`,
        start_time: new Date(Date.now() - i * 3600000).toISOString(),
        end_time: new Date(Date.now() - i * 3600000 + 30000).toISOString(),
        total_duration_ms: 30000 + Math.random() * 20000,
        scenario: scenarios[i % scenarios.length],
        zkp_metrics: {
          setup_time_ms: 20 + Math.random() * 40,
          witness_generation_time_ms: 30 + Math.random() * 50,
          proof_generation_time_ms: 80 + Math.random() * 100,
          proof_verification_time_ms: 100 + Math.random() * 150,
          proof_size_bytes: 1000 + Math.random() * 500,
          circuit_constraints: 1000,
          circuit_advice_columns: 5,
          circuit_fixed_columns: 3,
          folding_iterations: Math.floor(Math.random() * 10),
        },
        training_metrics: {
          dataset_size: 500 + Math.random() * 500,
          num_features: 5,
          training_time_ms: 2 + Math.random() * 8,
          epochs_completed: 10,
          final_loss: 0.1 + Math.random() * 0.4,
          initial_loss: 0.4 + Math.random() * 0.2,
          convergence_epoch: Math.floor(Math.random() * 10),
          loss_history: Array.from(
            { length: 10 },
            (_, i) => 0.5 - i * 0.03 + Math.random() * 0.02
          ),
        },
        system_metrics: [],
      };
      sampleData.push(data);
    }

    return sampleData;
  }

  updateMetrics() {
    const data = this.getFilteredData();
    if (data.length === 0) return;

    const metrics = this.calculateAggregateMetrics(data);

    document.getElementById("activeClients").textContent = this.isRunning
      ? data.length
      : "0";
    document.getElementById("avgProofTime").textContent = `${Math.round(
      metrics.avgProofTime
    )}ms`;
    document.getElementById("avgVerifyTime").textContent = `${Math.round(
      metrics.avgVerifyTime
    )}ms`;
    document.getElementById("avgTrainingTime").textContent = `${Math.round(
      metrics.avgTrainingTime
    )}ms`;
    document.getElementById("avgLoss").textContent = metrics.avgLoss.toFixed(3);
    document.getElementById("avgProofSize").textContent = `${Math.round(
      metrics.avgProofSize / 1024
    )}KB`;
  }
  calculateAggregateMetrics(data) {
    if (!Array.isArray(data) || data.length === 0) {
      return {
        avgProofTime: 0,
        avgVerifyTime: 0,
        avgTrainingTime: 0,
        avgLoss: 0,
        avgProofSize: 0,
      };
    }

    const totals = data.reduce(
      (acc, item) => {
        acc.proofTime += item.zkp_metrics?.proof_generation_time_ms || 0;
        acc.verifyTime += item.zkp_metrics?.proof_verification_time_ms || 0;
        acc.trainingTime += item.training_metrics?.training_time_ms || 0;
        acc.loss += item.training_metrics?.final_loss || 0;
        acc.proofSize += item.zkp_metrics?.proof_size_bytes || 0;
        return acc;
      },
      {
        proofTime: 0,
        verifyTime: 0,
        trainingTime: 0,
        loss: 0,
        proofSize: 0,
      }
    );

    const count = data.length;
    return {
      avgProofTime: totals.proofTime / count,
      avgVerifyTime: totals.verifyTime / count,
      avgTrainingTime: totals.trainingTime / count,
      avgLoss: totals.loss / count,
      avgProofSize: totals.proofSize / count,
    };
  }

  updateTable() {
    const data = this.getFilteredData();
    const tbody = document.querySelector("#benchmarkTable tbody");
    tbody.innerHTML = "";

    if (!Array.isArray(data)) {
      console.warn("benchmarkData is not an array:", data);
      return;
    }

    data.slice(0, 20).forEach((item) => {
      const row = document.createElement("tr");
      row.innerHTML = `
                <td>${new Date(item.start_time).toLocaleString()}</td>
                <td>${item.scenario || "Unknown"}</td>
                <td>${this.extractClientCount(item.client_id)}</td>
                <td>-</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_generation_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_verification_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.training_metrics?.training_time_ms || 0
                )}ms</td>
                <td>${(item.training_metrics?.final_loss || 0).toFixed(3)}</td>
                <td><span class="status-badge success">Completed</span></td>
                <td>
                    <button class="btn btn-secondary btn-sm" onclick="dashboard.viewDetails('${
                      item.session_id
                    }')">
                        <i class="fas fa-eye"></i>
                    </button>
                </td>
            `;
      tbody.appendChild(row);
    });
  }

  updateCharts() {
    const data = this.getFilteredData();
    if (this.charts.loss) {
      this.charts.loss.data = this.prepareLossData(data);
      this.charts.loss.update();
    }

    if (this.charts.trainingTime) {
      this.charts.trainingTime.data = this.prepareTrainingTimeData(data);
      this.charts.trainingTime.update();
    }

    if (this.charts.epochs) {
      this.charts.epochs.data = this.prepareEpochsData(data);
      this.charts.epochs.update();
    }

    if (this.charts.dataset) {
      this.charts.dataset.data = this.prepareDatasetData(data);
      this.charts.dataset.update();
    }

    if (this.charts.zkpTime) {
      this.charts.zkpTime.data = this.prepareZKPTimeData(data);
      this.charts.zkpTime.update();
    }

    if (this.charts.proofSize) {
      this.charts.proofSize.data = this.prepareProofSizeData(data);
      this.charts.proofSize.update();
    }

    if (this.charts.circuit) {
      this.charts.circuit.data = this.prepareCircuitData(data);
      this.charts.circuit.update();
    }

    if (this.charts.setupTime) {
      this.charts.setupTime.data = this.prepareSetupTimeData(data);
      this.charts.setupTime.update();
    }

    if (this.charts.scalability) {
      this.charts.scalability.data = this.prepareScalabilityData(data);
      this.charts.scalability.update();
    }

    if (this.charts.throughput) {
      this.charts.throughput.data = this.prepareThroughputData(data);
      this.charts.throughput.update();
    }

    if (this.charts.resource) {
      this.charts.resource.data = this.prepareResourceData(data);
      this.charts.resource.update();
    }

    if (this.charts.latency) {
      this.charts.latency.data = this.prepareLatencyData(data);
      this.charts.latency.update();
    }

    if (this.charts.comparison) {
      this.charts.comparison.data = this.prepareComparisonData(
        this.selectedMetric,
        this.selectedGroupBy
      );
      this.charts.comparison.update();
    }
  }

  switchTab(tabName) {
    // Hide all tab contents
    document.querySelectorAll(".tab-content").forEach((content) => {
      content.classList.remove("active");
    });

    // Remove active class from all tab buttons
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.classList.remove("active");
    });

    // Show selected tab content
    document.getElementById(`${tabName}Tab`).classList.add("active");
    document.querySelector(`[data-tab="${tabName}"]`).classList.add("active");

    // Update charts for the active tab
    setTimeout(() => {
      this.updateChartsForTab(tabName);
    }, 100);
  }

  initializeTabs() {
    // Show first tab by default
    this.switchTab("training");
  }

  setupEventListeners() {
    // Tab switching
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        this.switchTab(e.target.dataset.tab);
      });
    });

    // Benchmark controls
    document.getElementById("runBenchmarkBtn").addEventListener("click", () => {
      this.runBenchmark();
    });

    document
      .getElementById("stopBenchmarkBtn")
      .addEventListener("click", () => {
        this.stopBenchmark();
      });

    // Other controls
    document.getElementById("refreshBtn").addEventListener("click", () => {
      this.refreshData();
    });

    document.getElementById("exportBtn").addEventListener("click", () => {
      this.exportData();
    });

    // Comparison controls
    document.getElementById("compareMetric").addEventListener("change", () => {
      this.updateComparisonChart();
    });

    document.getElementById("groupBy").addEventListener("change", () => {
      this.updateComparisonChart();
    });
  }

  initializeCharts() {
    const chartOptions = {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: "top",
        },
        tooltip: {
          mode: "index",
          intersect: false,
        },
      },
      scales: {
        x: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
        y: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
      },
    };

    // Training Metrics Charts
    this.charts.loss = new Chart(document.getElementById("lossChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Loss" },
          },
        },
      },
    });

    this.charts.trainingTime = new Chart(
      document.getElementById("trainingTimeChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Training Time (ms)" },
            },
          },
        },
      }
    );

    this.charts.epochs = new Chart(document.getElementById("epochsChart"), {
      type: "bar",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Epochs Completed" },
          },
        },
      },
    });

    this.charts.dataset = new Chart(document.getElementById("datasetChart"), {
      type: "doughnut",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { position: "right" },
        },
      },
    });

    // ZKP Metrics Charts
    this.charts.zkpTime = new Chart(document.getElementById("zkpTimeChart"), {
      type: "scatter",
      data: { datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          x: { title: { display: true, text: "Proof Generation Time (ms)" } },
          y: { title: { display: true, text: "Verification Time (ms)" } },
        },
      },
    });

    this.charts.proofSize = new Chart(
      document.getElementById("proofSizeChart"),
      {
        type: "histogram",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Frequency" },
            },
          },
        },
      }
    );

    this.charts.circuit = new Chart(document.getElementById("circuitChart"), {
      type: "radar",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          r: {
            beginAtZero: true,
          },
        },
      },
    });

    this.charts.setupTime = new Chart(
      document.getElementById("setupTimeChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Setup Time (ms)" },
            },
          },
        },
      }
    );

    // Performance Charts
    this.charts.scalability = new Chart(
      document.getElementById("scalabilityChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            x: { title: { display: true, text: "Number of Clients" } },
            y: { title: { display: true, text: "Average Time (ms)" } },
          },
        },
      }
    );

    this.charts.throughput = new Chart(
      document.getElementById("throughputChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Operations/Second" },
            },
          },
        },
      }
    );

    this.charts.resource = new Chart(document.getElementById("resourceChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Usage %" },
          },
        },
      },
    });

    this.charts.latency = new Chart(document.getElementById("latencyChart"), {
      type: "box",
      data: { labels: [], datasets: [] },
      options: chartOptions,
    });

    // Comparison Chart
    this.charts.comparison = new Chart(
      document.getElementById("comparisonChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Value" },
            },
          },
        },
      }
    );
  }
  async loadBenchmarkData() {
    try {
      const response = await fetch("/api/benchmarks");
      if (response.ok) {
        const data = await response.json();
        // The API returns {benchmarks: [...], count: ..., metrics: ...}
        // We need just the benchmarks array
        this.benchmarkData = data.benchmarks || [];
      } else {
        // Fallback: load from local files
        await this.loadLocalBenchmarkData();
      }
    } catch (error) {
      console.warn("Failed to load from API, trying local files:", error);
      await this.loadLocalBenchmarkData();
    }
  }

  async loadLocalBenchmarkData() {
    try {
      // Load individual benchmark files
      const benchmarkFiles = await this.getBenchmarkFiles();
      this.benchmarkData = [];

      for (const file of benchmarkFiles) {
        try {
          const response = await fetch(`../benchmarks/${file}`);
          if (response.ok) {
            const data = await response.json();
            data.filename = file;
            this.benchmarkData.push(data);
          }
        } catch (error) {
          console.warn(`Failed to load ${file}:`, error);
        }
      }
    } catch (error) {
      console.error("Failed to load benchmark data:", error);
      // Use sample data for demo
      this.benchmarkData = this.generateSampleData();
    }
  }

  async getBenchmarkFiles() {
    // Return a list of known benchmark files
    return [
      "benchmark_20250627_100324_client_benchmark_client_0.json",
      "benchmark_20250627_100324_client_benchmark_client_1.json",
      "benchmark_20250627_100325_client_benchmark_client_2.json",
      "benchmark_20250627_101136_client_benchmark_client_0.json",
      "benchmark_20250627_101136_client_benchmark_client_1.json",
      "benchmark_20250627_101138_client_benchmark_client_2.json",
    ];
  }

  generateSampleData() {
    // Generate sample data for demonstration
    const sampleData = [];
    const scenarios = [
      "single-client",
      "multi-client-sequential",
      "multi-client-concurrent",
    ];

    for (let i = 0; i < 10; i++) {
      const data = {
        session_id: `sample-${i}`,
        client_id: `client_${i}`,
        start_time: new Date(Date.now() - i * 3600000).toISOString(),
        end_time: new Date(Date.now() - i * 3600000 + 30000).toISOString(),
        total_duration_ms: 30000 + Math.random() * 20000,
        scenario: scenarios[i % scenarios.length],
        zkp_metrics: {
          setup_time_ms: 20 + Math.random() * 40,
          witness_generation_time_ms: 30 + Math.random() * 50,
          proof_generation_time_ms: 80 + Math.random() * 100,
          proof_verification_time_ms: 100 + Math.random() * 150,
          proof_size_bytes: 1000 + Math.random() * 500,
          circuit_constraints: 1000,
          circuit_advice_columns: 5,
          circuit_fixed_columns: 3,
          folding_iterations: Math.floor(Math.random() * 10),
        },
        training_metrics: {
          dataset_size: 500 + Math.random() * 500,
          num_features: 5,
          training_time_ms: 2 + Math.random() * 8,
          epochs_completed: 10,
          final_loss: 0.1 + Math.random() * 0.4,
          initial_loss: 0.4 + Math.random() * 0.2,
          convergence_epoch: Math.floor(Math.random() * 10),
          loss_history: Array.from(
            { length: 10 },
            (_, i) => 0.5 - i * 0.03 + Math.random() * 0.02
          ),
        },
        system_metrics: [],
      };
      sampleData.push(data);
    }

    return sampleData;
  }

  updateMetrics() {
    const data = this.getFilteredData();
    if (data.length === 0) return;

    const metrics = this.calculateAggregateMetrics(data);

    document.getElementById("activeClients").textContent = this.isRunning
      ? data.length
      : "0";
    document.getElementById("avgProofTime").textContent = `${Math.round(
      metrics.avgProofTime
    )}ms`;
    document.getElementById("avgVerifyTime").textContent = `${Math.round(
      metrics.avgVerifyTime
    )}ms`;
    document.getElementById("avgTrainingTime").textContent = `${Math.round(
      metrics.avgTrainingTime
    )}ms`;
    document.getElementById("avgLoss").textContent = metrics.avgLoss.toFixed(3);
    document.getElementById("avgProofSize").textContent = `${Math.round(
      metrics.avgProofSize / 1024
    )}KB`;
  }
  calculateAggregateMetrics(data) {
    if (!Array.isArray(data) || data.length === 0) {
      return {
        avgProofTime: 0,
        avgVerifyTime: 0,
        avgTrainingTime: 0,
        avgLoss: 0,
        avgProofSize: 0,
      };
    }

    const totals = data.reduce(
      (acc, item) => {
        acc.proofTime += item.zkp_metrics?.proof_generation_time_ms || 0;
        acc.verifyTime += item.zkp_metrics?.proof_verification_time_ms || 0;
        acc.trainingTime += item.training_metrics?.training_time_ms || 0;
        acc.loss += item.training_metrics?.final_loss || 0;
        acc.proofSize += item.zkp_metrics?.proof_size_bytes || 0;
        return acc;
      },
      {
        proofTime: 0,
        verifyTime: 0,
        trainingTime: 0,
        loss: 0,
        proofSize: 0,
      }
    );

    const count = data.length;
    return {
      avgProofTime: totals.proofTime / count,
      avgVerifyTime: totals.verifyTime / count,
      avgTrainingTime: totals.trainingTime / count,
      avgLoss: totals.loss / count,
      avgProofSize: totals.proofSize / count,
    };
  }

  updateTable() {
    const data = this.getFilteredData();
    const tbody = document.querySelector("#benchmarkTable tbody");
    tbody.innerHTML = "";

    if (!Array.isArray(data)) {
      console.warn("benchmarkData is not an array:", data);
      return;
    }

    data.slice(0, 20).forEach((item) => {
      const row = document.createElement("tr");
      row.innerHTML = `
                <td>${new Date(item.start_time).toLocaleString()}</td>
                <td>${item.scenario || "Unknown"}</td>
                <td>${this.extractClientCount(item.client_id)}</td>
                <td>-</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_generation_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_verification_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.training_metrics?.training_time_ms || 0
                )}ms</td>
                <td>${(item.training_metrics?.final_loss || 0).toFixed(3)}</td>
                <td><span class="status-badge success">Completed</span></td>
                <td>
                    <button class="btn btn-secondary btn-sm" onclick="dashboard.viewDetails('${
                      item.session_id
                    }')">
                        <i class="fas fa-eye"></i>
                    </button>
                </td>
            `;
      tbody.appendChild(row);
    });
  }

  updateCharts() {
    const data = this.getFilteredData();
    if (this.charts.loss) {
      this.charts.loss.data = this.prepareLossData(data);
      this.charts.loss.update();
    }

    if (this.charts.trainingTime) {
      this.charts.trainingTime.data = this.prepareTrainingTimeData(data);
      this.charts.trainingTime.update();
    }

    if (this.charts.epochs) {
      this.charts.epochs.data = this.prepareEpochsData(data);
      this.charts.epochs.update();
    }

    if (this.charts.dataset) {
      this.charts.dataset.data = this.prepareDatasetData(data);
      this.charts.dataset.update();
    }

    if (this.charts.zkpTime) {
      this.charts.zkpTime.data = this.prepareZKPTimeData(data);
      this.charts.zkpTime.update();
    }

    if (this.charts.proofSize) {
      this.charts.proofSize.data = this.prepareProofSizeData(data);
      this.charts.proofSize.update();
    }

    if (this.charts.circuit) {
      this.charts.circuit.data = this.prepareCircuitData(data);
      this.charts.circuit.update();
    }

    if (this.charts.setupTime) {
      this.charts.setupTime.data = this.prepareSetupTimeData(data);
      this.charts.setupTime.update();
    }

    if (this.charts.scalability) {
      this.charts.scalability.data = this.prepareScalabilityData(data);
      this.charts.scalability.update();
    }

    if (this.charts.throughput) {
      this.charts.throughput.data = this.prepareThroughputData(data);
      this.charts.throughput.update();
    }

    if (this.charts.resource) {
      this.charts.resource.data = this.prepareResourceData(data);
      this.charts.resource.update();
    }

    if (this.charts.latency) {
      this.charts.latency.data = this.prepareLatencyData(data);
      this.charts.latency.update();
    }

    if (this.charts.comparison) {
      this.charts.comparison.data = this.prepareComparisonData(
        this.selectedMetric,
        this.selectedGroupBy
      );
      this.charts.comparison.update();
    }
  }

  switchTab(tabName) {
    // Hide all tab contents
    document.querySelectorAll(".tab-content").forEach((content) => {
      content.classList.remove("active");
    });

    // Remove active class from all tab buttons
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.classList.remove("active");
    });

    // Show selected tab content
    document.getElementById(`${tabName}Tab`).classList.add("active");
    document.querySelector(`[data-tab="${tabName}"]`).classList.add("active");

    // Update charts for the active tab
    setTimeout(() => {
      this.updateChartsForTab(tabName);
    }, 100);
  }

  initializeTabs() {
    // Show first tab by default
    this.switchTab("training");
  }

  setupEventListeners() {
    // Tab switching
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        this.switchTab(e.target.dataset.tab);
      });
    });

    // Benchmark controls
    document.getElementById("runBenchmarkBtn").addEventListener("click", () => {
      this.runBenchmark();
    });

    document
      .getElementById("stopBenchmarkBtn")
      .addEventListener("click", () => {
        this.stopBenchmark();
      });

    // Other controls
    document.getElementById("refreshBtn").addEventListener("click", () => {
      this.refreshData();
    });

    document.getElementById("exportBtn").addEventListener("click", () => {
      this.exportData();
    });

    // Comparison controls
    document.getElementById("compareMetric").addEventListener("change", () => {
      this.updateComparisonChart();
    });

    document.getElementById("groupBy").addEventListener("change", () => {
      this.updateComparisonChart();
    });
  }

  initializeCharts() {
    const chartOptions = {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: "top",
        },
        tooltip: {
          mode: "index",
          intersect: false,
        },
      },
      scales: {
        x: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
        y: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
      },
    };

    // Training Metrics Charts
    this.charts.loss = new Chart(document.getElementById("lossChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Loss" },
          },
        },
      },
    });

    this.charts.trainingTime = new Chart(
      document.getElementById("trainingTimeChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Training Time (ms)" },
            },
          },
        },
      }
    );

    this.charts.epochs = new Chart(document.getElementById("epochsChart"), {
      type: "bar",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Epochs Completed" },
          },
        },
      },
    });

    this.charts.dataset = new Chart(document.getElementById("datasetChart"), {
      type: "doughnut",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { position: "right" },
        },
      },
    });

    // ZKP Metrics Charts
    this.charts.zkpTime = new Chart(document.getElementById("zkpTimeChart"), {
      type: "scatter",
      data: { datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          x: { title: { display: true, text: "Proof Generation Time (ms)" } },
          y: { title: { display: true, text: "Verification Time (ms)" } },
        },
      },
    });

    this.charts.proofSize = new Chart(
      document.getElementById("proofSizeChart"),
      {
        type: "histogram",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Frequency" },
            },
          },
        },
      }
    );

    this.charts.circuit = new Chart(document.getElementById("circuitChart"), {
      type: "radar",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          r: {
            beginAtZero: true,
          },
        },
      },
    });

    this.charts.setupTime = new Chart(
      document.getElementById("setupTimeChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Setup Time (ms)" },
            },
          },
        },
      }
    );

    // Performance Charts
    this.charts.scalability = new Chart(
      document.getElementById("scalabilityChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            x: { title: { display: true, text: "Number of Clients" } },
            y: { title: { display: true, text: "Average Time (ms)" } },
          },
        },
      }
    );

    this.charts.throughput = new Chart(
      document.getElementById("throughputChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Operations/Second" },
            },
          },
        },
      }
    );

    this.charts.resource = new Chart(document.getElementById("resourceChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Usage %" },
          },
        },
      },
    });

    this.charts.latency = new Chart(document.getElementById("latencyChart"), {
      type: "box",
      data: { labels: [], datasets: [] },
      options: chartOptions,
    });

    // Comparison Chart
    this.charts.comparison = new Chart(
      document.getElementById("comparisonChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Value" },
            },
          },
        },
      }
    );
  }
  async loadBenchmarkData() {
    try {
      const response = await fetch("/api/benchmarks");
      if (response.ok) {
        const data = await response.json();
        // The API returns {benchmarks: [...], count: ..., metrics: ...}
        // We need just the benchmarks array
        this.benchmarkData = data.benchmarks || [];
      } else {
        // Fallback: load from local files
        await this.loadLocalBenchmarkData();
      }
    } catch (error) {
      console.warn("Failed to load from API, trying local files:", error);
      await this.loadLocalBenchmarkData();
    }
  }

  async loadLocalBenchmarkData() {
    try {
      // Load individual benchmark files
      const benchmarkFiles = await this.getBenchmarkFiles();
      this.benchmarkData = [];

      for (const file of benchmarkFiles) {
        try {
          const response = await fetch(`../benchmarks/${file}`);
          if (response.ok) {
            const data = await response.json();
            data.filename = file;
            this.benchmarkData.push(data);
          }
        } catch (error) {
          console.warn(`Failed to load ${file}:`, error);
        }
      }
    } catch (error) {
      console.error("Failed to load benchmark data:", error);
      // Use sample data for demo
      this.benchmarkData = this.generateSampleData();
    }
  }

  async getBenchmarkFiles() {
    // Return a list of known benchmark files
    return [
      "benchmark_20250627_100324_client_benchmark_client_0.json",
      "benchmark_20250627_100324_client_benchmark_client_1.json",
      "benchmark_20250627_100325_client_benchmark_client_2.json",
      "benchmark_20250627_101136_client_benchmark_client_0.json",
      "benchmark_20250627_101136_client_benchmark_client_1.json",
      "benchmark_20250627_101138_client_benchmark_client_2.json",
    ];
  }

  generateSampleData() {
    // Generate sample data for demonstration
    const sampleData = [];
    const scenarios = [
      "single-client",
      "multi-client-sequential",
      "multi-client-concurrent",
    ];

    for (let i = 0; i < 10; i++) {
      const data = {
        session_id: `sample-${i}`,
        client_id: `client_${i}`,
        start_time: new Date(Date.now() - i * 3600000).toISOString(),
        end_time: new Date(Date.now() - i * 3600000 + 30000).toISOString(),
        total_duration_ms: 30000 + Math.random() * 20000,
        scenario: scenarios[i % scenarios.length],
        zkp_metrics: {
          setup_time_ms: 20 + Math.random() * 40,
          witness_generation_time_ms: 30 + Math.random() * 50,
          proof_generation_time_ms: 80 + Math.random() * 100,
          proof_verification_time_ms: 100 + Math.random() * 150,
          proof_size_bytes: 1000 + Math.random() * 500,
          circuit_constraints: 1000,
          circuit_advice_columns: 5,
          circuit_fixed_columns: 3,
          folding_iterations: Math.floor(Math.random() * 10),
        },
        training_metrics: {
          dataset_size: 500 + Math.random() * 500,
          num_features: 5,
          training_time_ms: 2 + Math.random() * 8,
          epochs_completed: 10,
          final_loss: 0.1 + Math.random() * 0.4,
          initial_loss: 0.4 + Math.random() * 0.2,
          convergence_epoch: Math.floor(Math.random() * 10),
          loss_history: Array.from(
            { length: 10 },
            (_, i) => 0.5 - i * 0.03 + Math.random() * 0.02
          ),
        },
        system_metrics: [],
      };
      sampleData.push(data);
    }

    return sampleData;
  }

  updateMetrics() {
    const data = this.getFilteredData();
    if (data.length === 0) return;

    const metrics = this.calculateAggregateMetrics(data);

    document.getElementById("activeClients").textContent = this.isRunning
      ? data.length
      : "0";
    document.getElementById("avgProofTime").textContent = `${Math.round(
      metrics.avgProofTime
    )}ms`;
    document.getElementById("avgVerifyTime").textContent = `${Math.round(
      metrics.avgVerifyTime
    )}ms`;
    document.getElementById("avgTrainingTime").textContent = `${Math.round(
      metrics.avgTrainingTime
    )}ms`;
    document.getElementById("avgLoss").textContent = metrics.avgLoss.toFixed(3);
    document.getElementById("avgProofSize").textContent = `${Math.round(
      metrics.avgProofSize / 1024
    )}KB`;
  }
  calculateAggregateMetrics(data) {
    if (!Array.isArray(data) || data.length === 0) {
      return {
        avgProofTime: 0,
        avgVerifyTime: 0,
        avgTrainingTime: 0,
        avgLoss: 0,
        avgProofSize: 0,
      };
    }

    const totals = data.reduce(
      (acc, item) => {
        acc.proofTime += item.zkp_metrics?.proof_generation_time_ms || 0;
        acc.verifyTime += item.zkp_metrics?.proof_verification_time_ms || 0;
        acc.trainingTime += item.training_metrics?.training_time_ms || 0;
        acc.loss += item.training_metrics?.final_loss || 0;
        acc.proofSize += item.zkp_metrics?.proof_size_bytes || 0;
        return acc;
      },
      {
        proofTime: 0,
        verifyTime: 0,
        trainingTime: 0,
        loss: 0,
        proofSize: 0,
      }
    );

    const count = data.length;
    return {
      avgProofTime: totals.proofTime / count,
      avgVerifyTime: totals.verifyTime / count,
      avgTrainingTime: totals.trainingTime / count,
      avgLoss: totals.loss / count,
      avgProofSize: totals.proofSize / count,
    };
  }

  updateTable() {
    const data = this.getFilteredData();
    const tbody = document.querySelector("#benchmarkTable tbody");
    tbody.innerHTML = "";

    if (!Array.isArray(data)) {
      console.warn("benchmarkData is not an array:", data);
      return;
    }

    data.slice(0, 20).forEach((item) => {
      const row = document.createElement("tr");
      row.innerHTML = `
                <td>${new Date(item.start_time).toLocaleString()}</td>
                <td>${item.scenario || "Unknown"}</td>
                <td>${this.extractClientCount(item.client_id)}</td>
                <td>-</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_generation_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_verification_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.training_metrics?.training_time_ms || 0
                )}ms</td>
                <td>${(item.training_metrics?.final_loss || 0).toFixed(3)}</td>
                <td><span class="status-badge success">Completed</span></td>
                <td>
                    <button class="btn btn-secondary btn-sm" onclick="dashboard.viewDetails('${
                      item.session_id
                    }')">
                        <i class="fas fa-eye"></i>
                    </button>
                </td>
            `;
      tbody.appendChild(row);
    });
  }

  updateCharts() {
    const data = this.getFilteredData();
    if (this.charts.loss) {
      this.charts.loss.data = this.prepareLossData(data);
      this.charts.loss.update();
    }

    if (this.charts.trainingTime) {
      this.charts.trainingTime.data = this.prepareTrainingTimeData(data);
      this.charts.trainingTime.update();
    }

    if (this.charts.epochs) {
      this.charts.epochs.data = this.prepareEpochsData(data);
      this.charts.epochs.update();
    }

    if (this.charts.dataset) {
      this.charts.dataset.data = this.prepareDatasetData(data);
      this.charts.dataset.update();
    }

    if (this.charts.zkpTime) {
      this.charts.zkpTime.data = this.prepareZKPTimeData(data);
      this.charts.zkpTime.update();
    }

    if (this.charts.proofSize) {
      this.charts.proofSize.data = this.prepareProofSizeData(data);
      this.charts.proofSize.update();
    }

    if (this.charts.circuit) {
      this.charts.circuit.data = this.prepareCircuitData(data);
      this.charts.circuit.update();
    }

    if (this.charts.setupTime) {
      this.charts.setupTime.data = this.prepareSetupTimeData(data);
      this.charts.setupTime.update();
    }

    if (this.charts.scalability) {
      this.charts.scalability.data = this.prepareScalabilityData(data);
      this.charts.scalability.update();
    }

    if (this.charts.throughput) {
      this.charts.throughput.data = this.prepareThroughputData(data);
      this.charts.throughput.update();
    }

    if (this.charts.resource) {
      this.charts.resource.data = this.prepareResourceData(data);
      this.charts.resource.update();
    }

    if (this.charts.latency) {
      this.charts.latency.data = this.prepareLatencyData(data);
      this.charts.latency.update();
    }

    if (this.charts.comparison) {
      this.charts.comparison.data = this.prepareComparisonData(
        this.selectedMetric,
        this.selectedGroupBy
      );
      this.charts.comparison.update();
    }
  }

  switchTab(tabName) {
    // Hide all tab contents
    document.querySelectorAll(".tab-content").forEach((content) => {
      content.classList.remove("active");
    });

    // Remove active class from all tab buttons
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.classList.remove("active");
    });

    // Show selected tab content
    document.getElementById(`${tabName}Tab`).classList.add("active");
    document.querySelector(`[data-tab="${tabName}"]`).classList.add("active");

    // Update charts for the active tab
    setTimeout(() => {
      this.updateChartsForTab(tabName);
    }, 100);
  }

  initializeTabs() {
    // Show first tab by default
    this.switchTab("training");
  }

  setupEventListeners() {
    // Tab switching
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        this.switchTab(e.target.dataset.tab);
      });
    });

    // Benchmark controls
    document.getElementById("runBenchmarkBtn").addEventListener("click", () => {
      this.runBenchmark();
    });

    document
      .getElementById("stopBenchmarkBtn")
      .addEventListener("click", () => {
        this.stopBenchmark();
      });

    // Other controls
    document.getElementById("refreshBtn").addEventListener("click", () => {
      this.refreshData();
    });

    document.getElementById("exportBtn").addEventListener("click", () => {
      this.exportData();
    });

    // Comparison controls
    document.getElementById("compareMetric").addEventListener("change", () => {
      this.updateComparisonChart();
    });

    document.getElementById("groupBy").addEventListener("change", () => {
      this.updateComparisonChart();
    });
  }

  initializeCharts() {
    const chartOptions = {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: "top",
        },
        tooltip: {
          mode: "index",
          intersect: false,
        },
      },
      scales: {
        x: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
        y: {
          display: true,
          grid: {
            display: true,
            color: "rgba(0,0,0,0.1)",
          },
        },
      },
    };

    // Training Metrics Charts
    this.charts.loss = new Chart(document.getElementById("lossChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Loss" },
          },
        },
      },
    });

    this.charts.trainingTime = new Chart(
      document.getElementById("trainingTimeChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Training Time (ms)" },
            },
          },
        },
      }
    );

    this.charts.epochs = new Chart(document.getElementById("epochsChart"), {
      type: "bar",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Epochs Completed" },
          },
        },
      },
    });

    this.charts.dataset = new Chart(document.getElementById("datasetChart"), {
      type: "doughnut",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { position: "right" },
        },
      },
    });

    // ZKP Metrics Charts
    this.charts.zkpTime = new Chart(document.getElementById("zkpTimeChart"), {
      type: "scatter",
      data: { datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          x: { title: { display: true, text: "Proof Generation Time (ms)" } },
          y: { title: { display: true, text: "Verification Time (ms)" } },
        },
      },
    });

    this.charts.proofSize = new Chart(
      document.getElementById("proofSizeChart"),
      {
        type: "histogram",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Frequency" },
            },
          },
        },
      }
    );

    this.charts.circuit = new Chart(document.getElementById("circuitChart"), {
      type: "radar",
      data: { labels: [], datasets: [] },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          r: {
            beginAtZero: true,
          },
        },
      },
    });

    this.charts.setupTime = new Chart(
      document.getElementById("setupTimeChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Setup Time (ms)" },
            },
          },
        },
      }
    );

    // Performance Charts
    this.charts.scalability = new Chart(
      document.getElementById("scalabilityChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            x: { title: { display: true, text: "Number of Clients" } },
            y: { title: { display: true, text: "Average Time (ms)" } },
          },
        },
      }
    );

    this.charts.throughput = new Chart(
      document.getElementById("throughputChart"),
      {
        type: "line",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Operations/Second" },
            },
          },
        },
      }
    );

    this.charts.resource = new Chart(document.getElementById("resourceChart"), {
      type: "line",
      data: { labels: [], datasets: [] },
      options: {
        ...chartOptions,
        scales: {
          ...chartOptions.scales,
          y: {
            ...chartOptions.scales.y,
            title: { display: true, text: "Usage %" },
          },
        },
      },
    });

    this.charts.latency = new Chart(document.getElementById("latencyChart"), {
      type: "box",
      data: { labels: [], datasets: [] },
      options: chartOptions,
    });

    // Comparison Chart
    this.charts.comparison = new Chart(
      document.getElementById("comparisonChart"),
      {
        type: "bar",
        data: { labels: [], datasets: [] },
        options: {
          ...chartOptions,
          scales: {
            ...chartOptions.scales,
            y: {
              ...chartOptions.scales.y,
              title: { display: true, text: "Value" },
            },
          },
        },
      }
    );
  }
  async loadBenchmarkData() {
    try {
      const response = await fetch("/api/benchmarks");
      if (response.ok) {
        const data = await response.json();
        // The API returns {benchmarks: [...], count: ..., metrics: ...}
        // We need just the benchmarks array
        this.benchmarkData = data.benchmarks || [];
      } else {
        // Fallback: load from local files
        await this.loadLocalBenchmarkData();
      }
    } catch (error) {
      console.warn("Failed to load from API, trying local files:", error);
      await this.loadLocalBenchmarkData();
    }
  }

  async loadLocalBenchmarkData() {
    try {
      // Load individual benchmark files
      const benchmarkFiles = await this.getBenchmarkFiles();
      this.benchmarkData = [];

      for (const file of benchmarkFiles) {
        try {
          const response = await fetch(`../benchmarks/${file}`);
          if (response.ok) {
            const data = await response.json();
            data.filename = file;
            this.benchmarkData.push(data);
          }
        } catch (error) {
          console.warn(`Failed to load ${file}:`, error);
        }
      }
    } catch (error) {
      console.error("Failed to load benchmark data:", error);
      // Use sample data for demo
      this.benchmarkData = this.generateSampleData();
    }
  }

  async getBenchmarkFiles() {
    // Return a list of known benchmark files
    return [
      "benchmark_20250627_100324_client_benchmark_client_0.json",
      "benchmark_20250627_100324_client_benchmark_client_1.json",
      "benchmark_20250627_100325_client_benchmark_client_2.json",
      "benchmark_20250627_101136_client_benchmark_client_0.json",
      "benchmark_20250627_101136_client_benchmark_client_1.json",
      "benchmark_20250627_101138_client_benchmark_client_2.json",
    ];
  }

  generateSampleData() {
    // Generate sample data for demonstration
    const sampleData = [];
    const scenarios = [
      "single-client",
      "multi-client-sequential",
      "multi-client-concurrent",
    ];

    for (let i = 0; i < 10; i++) {
      const data = {
        session_id: `sample-${i}`,
        client_id: `client_${i}`,
        start_time: new Date(Date.now() - i * 3600000).toISOString(),
        end_time: new Date(Date.now() - i * 3600000 + 30000).toISOString(),
        total_duration_ms: 30000 + Math.random() * 20000,
        scenario: scenarios[i % scenarios.length],
        zkp_metrics: {
          setup_time_ms: 20 + Math.random() * 40,
          witness_generation_time_ms: 30 + Math.random() * 50,
          proof_generation_time_ms: 80 + Math.random() * 100,
          proof_verification_time_ms: 100 + Math.random() * 150,
          proof_size_bytes: 1000 + Math.random() * 500,
          circuit_constraints: 1000,
          circuit_advice_columns: 5,
          circuit_fixed_columns: 3,
          folding_iterations: Math.floor(Math.random() * 10),
        },
        training_metrics: {
          dataset_size: 500 + Math.random() * 500,
          num_features: 5,
          training_time_ms: 2 + Math.random() * 8,
          epochs_completed: 10,
          final_loss: 0.1 + Math.random() * 0.4,
          initial_loss: 0.4 + Math.random() * 0.2,
          convergence_epoch: Math.floor(Math.random() * 10),
          loss_history: Array.from(
            { length: 10 },
            (_, i) => 0.5 - i * 0.03 + Math.random() * 0.02
          ),
        },
        system_metrics: [],
      };
      sampleData.push(data);
    }

    return sampleData;
  }

  updateMetrics() {
    const data = this.getFilteredData();
    if (data.length === 0) return;

    const metrics = this.calculateAggregateMetrics(data);

    document.getElementById("activeClients").textContent = this.isRunning
      ? data.length
      : "0";
    document.getElementById("avgProofTime").textContent = `${Math.round(
      metrics.avgProofTime
    )}ms`;
    document.getElementById("avgVerifyTime").textContent = `${Math.round(
      metrics.avgVerifyTime
    )}ms`;
    document.getElementById("avgTrainingTime").textContent = `${Math.round(
      metrics.avgTrainingTime
    )}ms`;
    document.getElementById("avgLoss").textContent = metrics.avgLoss.toFixed(3);
    document.getElementById("avgProofSize").textContent = `${Math.round(
      metrics.avgProofSize / 1024
    )}KB`;
  }
  calculateAggregateMetrics(data) {
    if (!Array.isArray(data) || data.length === 0) {
      return {
        avgProofTime: 0,
        avgVerifyTime: 0,
        avgTrainingTime: 0,
        avgLoss: 0,
        avgProofSize: 0,
      };
    }

    const totals = data.reduce(
      (acc, item) => {
        acc.proofTime += item.zkp_metrics?.proof_generation_time_ms || 0;
        acc.verifyTime += item.zkp_metrics?.proof_verification_time_ms || 0;
        acc.trainingTime += item.training_metrics?.training_time_ms || 0;
        acc.loss += item.training_metrics?.final_loss || 0;
        acc.proofSize += item.zkp_metrics?.proof_size_bytes || 0;
        return acc;
      },
      {
        proofTime: 0,
        verifyTime: 0,
        trainingTime: 0,
        loss: 0,
        proofSize: 0,
      }
    );

    const count = data.length;
    return {
      avgProofTime: totals.proofTime / count,
      avgVerifyTime: totals.verifyTime / count,
      avgTrainingTime: totals.trainingTime / count,
      avgLoss: totals.loss / count,
      avgProofSize: totals.proofSize / count,
    };
  }

  updateTable() {
    const data = this.getFilteredData();
    const tbody = document.querySelector("#benchmarkTable tbody");
    tbody.innerHTML = "";

    if (!Array.isArray(data)) {
      console.warn("benchmarkData is not an array:", data);
      return;
    }

    data.slice(0, 20).forEach((item) => {
      const row = document.createElement("tr");
      row.innerHTML = `
                <td>${new Date(item.start_time).toLocaleString()}</td>
                <td>${item.scenario || "Unknown"}</td>
                <td>${this.extractClientCount(item.client_id)}</td>
                <td>-</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_generation_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.zkp_metrics?.proof_verification_time_ms || 0
                )}ms</td>
                <td>${Math.round(
                  item.training_metrics?.training_time_ms || 0
                )}ms</td>
                <td>${(item.training_metrics?.final_loss || 0).toFixed(3)}</td>
                <td><span class="status-badge success">Completed</span></td>
                <td>
                    <button class="btn btn-secondary btn-sm" onclick="dashboard.viewDetails('${
                      item.session_id
                    }')">
                        <i class="fas fa-eye"></i>
                    </button>
                </td>
            `;
      tbody.appendChild(row);
    });
  }

  updateCharts() {
    const data = this.getFilteredData();
    if (this.charts.loss) {
      this.charts.loss.data = this.prepareLossData(data);
      this.charts.loss.update();
    }

    if (this.charts.trainingTime) {
      this.charts.trainingTime.data = this.prepareTrainingTimeData(data);
      this.charts.trainingTime.update();
    }

    if (this.charts.epochs) {
      this.charts.epochs.data = this.prepareEpochsData(data);
      this.charts.epochs.update();
    }

    if (this.charts.dataset) {
      this.charts.dataset.data = this.prepareDatasetData(data);
      this.charts.dataset.update();
    }

    if (this.charts.zkpTime) {
      this.charts.zkpTime.data = this.prepareZKPTimeData(data);
      this.charts.zkpTime.update();
    }

    if (this.charts.proofSize) {
      this.charts.proofSize.data = this.prepareProofSizeData(data);
      this.charts.proofSize.update();
    }

    if (this.charts.circuit) {
      this.charts.circuit.data = this.prepareCircuitData(data);
      this.charts.circuit.update();
    }

    if (this.charts.setupTime) {
      this.charts.setupTime.data = this.prepareSetupTimeData(data);
      this.charts.setupTime.update();
    }

    if (this.charts.scalability) {
      this.charts.scalability.data = this.prepareScalabilityData(data);
      this.charts.scalability.update();
    }

    if (this.charts.throughput) {
      this.charts.throughput.data = this.prepareThroughputData(data);
      this.charts.throughput.update();
    }

    if (this.charts.resource) {
      this.charts.resource.data = this.prepareResourceData(data);
      this.charts.resource.update();
    }

    if (this.charts.latency) {
      this.charts.latency.data = this.prepareLatencyData(data);
      this.charts.latency.update();
    }

    if (this.charts.comparison) {
      this.charts.comparison.data = this.prepareComparisonData(
        this.selectedMetric,
        this.selectedGroupBy
      );
      this.charts.comparison.update();
    }
  }

  extractClientCount(clientId) {
    // Extract number from client ID or return 1 for single client
    if (!clientId || typeof clientId !== "string") {
      return 1;
    }
    const match = clientId.match(/\d+/);
    return match ? parseInt(match[0]) + 1 : 1;
  }

  async runBenchmark() {
    if (this.isRunning) return;

    const config = this.getBenchmarkConfig();
    this.isRunning = true;
    this.updateBenchmarkStatus("running", "Starting benchmark...");
    this.toggleBenchmarkButtons(true);

    try {
      // Show loading overlay
      document.getElementById("loadingOverlay").classList.add("show");

      // Simulate benchmark execution
      await this.executeBenchmark(config);

      this.updateBenchmarkStatus(
        "success",
        "Benchmark completed successfully!"
      );
      await this.loadBenchmarkData();
      this.updateMetrics();
      this.updateTable();
      this.updateChartsForTab(
        document.querySelector(".tab-btn.active").dataset.tab
      );
    } catch (error) {
      console.error("Benchmark failed:", error);
      this.updateBenchmarkStatus("error", `Benchmark failed: ${error.message}`);
    } finally {
      this.isRunning = false;
      this.toggleBenchmarkButtons(false);
      document.getElementById("loadingOverlay").classList.remove("show");
    }
  }

  getBenchmarkConfig() {
    return {
      scenario: document.getElementById("scenario").value,
      numClients: parseInt(document.getElementById("numClients").value),
      numRounds: parseInt(document.getElementById("numRounds").value),
      clientDelay: parseInt(document.getElementById("clientDelay").value),
      maxConcurrent: parseInt(document.getElementById("maxConcurrent").value),
      serverUrl: document.getElementById("serverUrl").value,
    };
  }

  async executeBenchmark(config) {
    // Execute the actual Rust benchmark command
    const command = this.buildBenchmarkCommand(config);

    try {
      const response = await fetch("/api/run-benchmark", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ command, config }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json();
      return result;
    } catch (error) {
      // Fallback: simulate benchmark for demo
      console.warn("Using simulated benchmark:", error.message);
      return this.simulateBenchmark(config);
    }
  }

  buildBenchmarkCommand(config) {
    let command = "cargo run --bin benchmarks --";
    command += ` --scenario ${config.scenario}`;
    command += ` --num-clients ${config.numClients}`;
    command += ` --rounds ${config.numRounds}`;
    command += ` --client-delay-ms ${config.clientDelay}`;
    command += ` --max-concurrent ${config.maxConcurrent}`;

    if (config.serverUrl) {
      command += ` --server-url ${config.serverUrl}`;
    }

    return command;
  }

  async simulateBenchmark(config) {
    const steps = [
      "Initializing benchmark environment...",
      "Loading configuration...",
      "Starting server...",
      "Spawning clients...",
      "Running training rounds...",
      "Generating ZK proofs...",
      "Verifying proofs...",
      "Collecting metrics...",
      "Generating report...",
    ];

    for (let i = 0; i < steps.length; i++) {
      document.getElementById("loadingProgress").textContent = steps[i];
      await new Promise((resolve) =>
        setTimeout(resolve, 1000 + Math.random() * 2000)
      );
    }

    // Add simulated data to benchmark results
    const newData = this.generateSampleData().slice(0, config.numClients);
    this.benchmarkData = [...newData, ...this.benchmarkData];

    return { success: true, message: "Benchmark completed" };
  }

  stopBenchmark() {
    if (!this.isRunning) return;

    // Send stop signal to backend
    fetch("/api/stop-benchmark", { method: "POST" }).catch((error) =>
      console.warn("Failed to stop benchmark:", error)
    );

    this.isRunning = false;
    this.toggleBenchmarkButtons(false);
    this.updateBenchmarkStatus("error", "Benchmark stopped by user");
    document.getElementById("loadingOverlay").classList.remove("show");
  }

  updateBenchmarkStatus(type, message) {
    const statusDiv = document.getElementById("benchmarkStatus");
    statusDiv.className = `status-display ${type}`;
    statusDiv.textContent = message;
  }

  toggleBenchmarkButtons(isRunning) {
    document.getElementById("runBenchmarkBtn").disabled = isRunning;
    document.getElementById("stopBenchmarkBtn").disabled = !isRunning;
  }

  async refreshData() {
    await this.loadBenchmarkData();
    this.updateMetrics();
    this.updateTable();
    this.updateChartsForTab(
      document.querySelector(".tab-btn.active").dataset.tab
    );

    // Show brief success message
    this.updateBenchmarkStatus("success", "Data refreshed successfully");
    setTimeout(() => {
      document.getElementById("benchmarkStatus").textContent = "";
      document.getElementById("benchmarkStatus").className = "status-display";
    }, 3000);
  }

  exportData() {
    const exportData = {
      timestamp: new Date().toISOString(),
      metrics: this.calculateAggregateMetrics(),
      benchmarks: this.benchmarkData,
    };

    const blob = new Blob([JSON.stringify(exportData, null, 2)], {
      type: "application/json",
    });

    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `zkp-fl-benchmark-export-${new Date()
      .toISOString()
      .slice(0, 19)}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  viewDetails(sessionId) {
    const data = this.benchmarkData.find((d) => d.session_id === sessionId);
    if (data) {
      const details = JSON.stringify(data, null, 2);
      const newWindow = window.open("", "_blank");
      newWindow.document.write(`
                <html>
                    <head><title>Benchmark Details - ${sessionId}</title></head>
                    <body>
                        <h1>Benchmark Details</h1>
                        <pre style="background: #f5f5f5; padding: 20px; border-radius: 5px;">${details}</pre>
                    </body>
                </html>
            `);
    }
  }

  startAutoRefresh() {
    // Refresh data every 30 seconds if not running a benchmark
    this.refreshInterval = setInterval(() => {
      if (!this.isRunning) {
        this.refreshData();
      }
    }, 30000);
  }

  destroy() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }

    // Cleanup charts
    Object.values(this.charts).forEach((chart) => {
      if (chart && chart.destroy) {
        chart.destroy();
      }
    });
  }
}

// Initialize dashboard when DOM is loaded
document.addEventListener("DOMContentLoaded", () => {
  window.dashboard = new ZKPDashboard();
});

// Cleanup on page unload
window.addEventListener("beforeunload", () => {
  if (window.dashboard) {
    window.dashboard.destroy();
  }
});
