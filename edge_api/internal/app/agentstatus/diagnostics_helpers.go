package agentstatus

import (
	"fmt"
	"sort"
	"strings"

	"github.com/example/dorohedoro/internal/model"
)

func (s *Service) presentDiagnostics(snapshot *diagnosticsSnapshot) diagnosticsPresentation {
	out := diagnosticsPresentation{
		Summary: model.HostAgentDiagnosticsSummary{
			DoctorStatus:    "unknown",
			SpoolStatus:     "unknown",
			TransportStatus: "unknown",
			TLSStatus:       "unknown",
			SourceStatus:    "unknown",
			TopIssues:       []model.AgentIssue{},
		},
		Checks:   []model.AgentDiagnosticsCheck{},
		Issues:   []model.AgentIssue{},
		Errors:   []model.AgentIssue{},
		Warnings: []model.AgentIssue{},
	}
	if snapshot == nil {
		out.Checks = append(out.Checks, model.AgentDiagnosticsCheck{
			CheckID:  "diagnostics",
			Name:     "Diagnostics snapshot",
			Status:   "unknown",
			Severity: "warn",
			Domain:   "runtime",
			Message:  "Диагностический snapshot еще не поступал.",
			Hint:     "Проверить, что агент отправляет runtime diagnostics через edge-api.",
		})
		return out
	}

	runtimeStatus := strings.ToLower(strings.TrimSpace(snapshot.RuntimeStatus))
	runtimeCheckStatus := "unknown"
	runtimeSeverity := "warn"
	runtimeMessage := "Runtime snapshot получен, но статус не заполнен."
	switch {
	case runtimeStatus == "" || runtimeStatus == "unknown":
	case strings.Contains(runtimeStatus, "healthy"), strings.Contains(runtimeStatus, "running"), strings.Contains(runtimeStatus, "ready"):
		runtimeCheckStatus = "pass"
		runtimeSeverity = "info"
		runtimeMessage = "Runtime агента сообщает healthy состояние."
	case snapshot.BlockedDelivery, strings.Contains(runtimeStatus, "fail"), strings.Contains(runtimeStatus, "error"), strings.Contains(runtimeStatus, "blocked"):
		runtimeCheckStatus = "fail"
		runtimeSeverity = "error"
		runtimeMessage = firstNonEmpty(ptrValue(snapshot.RuntimeStatusReason), ptrValue(snapshot.BlockedReason), "Runtime агента сообщает блокирующую ошибку.")
	default:
		runtimeCheckStatus = "warn"
		runtimeSeverity = "warn"
		runtimeMessage = firstNonEmpty(ptrValue(snapshot.RuntimeStatusReason), ptrValue(snapshot.DegradedReason), "Runtime агента работает в деградированном режиме.")
	}
	out.Checks = append(out.Checks, model.AgentDiagnosticsCheck{
		CheckID:  "runtime",
		Name:     "Runtime state",
		Status:   runtimeCheckStatus,
		Severity: runtimeSeverity,
		Domain:   "runtime",
		Message:  runtimeMessage,
	})
	if snapshot.LastError != nil {
		domain := issueDomainFromErrorKind(ptrValue(snapshot.LastErrorKind))
		if domain == "unknown" {
			domain = "runtime"
		}
		out.Issues = append(out.Issues, model.AgentIssue{
			Code:     firstNonEmpty(ptrValue(snapshot.LastErrorKind), "runtime_last_error"),
			Severity: "error",
			Domain:   domain,
			Message:  *snapshot.LastError,
		})
	}
	if snapshot.DegradedMode {
		out.Issues = append(out.Issues, model.AgentIssue{
			Code:     "runtime_degraded",
			Severity: "warn",
			Domain:   "runtime",
			Message:  firstNonEmpty(ptrValue(snapshot.DegradedReason), "Runtime агента переведен в degraded mode."),
		})
		out.Degraded = model.AgentDegradedMode{
			Active: true,
			Reason: ptrValue(snapshot.DegradedReason),
		}
	}

	identityStatus := strings.ToLower(strings.TrimSpace(snapshot.IdentityStatus.Status))
	if identityStatus == "" {
		identityStatus = "unknown"
	}
	identityCheckStatus := "pass"
	identitySeverity := "info"
	identityMessage := "Identity и enrollment состояние выглядят корректно."
	if identityStatus == "unknown" {
		identityCheckStatus = "unknown"
		identitySeverity = "warn"
		identityMessage = "Identity status пока не прислан агентом."
	} else if identityStatus != "ok" && identityStatus != "healthy" && identityStatus != "enrolled" {
		identityCheckStatus = "warn"
		identitySeverity = "warn"
		identityMessage = firstNonEmpty(ptrValue(snapshot.IdentityStatus.Reason), fmt.Sprintf("Identity status reported as %s.", identityStatus))
		out.Issues = append(out.Issues, model.AgentIssue{
			Code:     "identity_status",
			Severity: "warn",
			Domain:   "enrollment",
			Message:  identityMessage,
		})
	}
	out.Checks = append(out.Checks, model.AgentDiagnosticsCheck{
		CheckID:  "identity",
		Name:     "Identity and enrollment",
		Status:   identityCheckStatus,
		Severity: identitySeverity,
		Domain:   "enrollment",
		Message:  identityMessage,
	})

	connectivityChecks, connectivityIssues, transportStatus, tlsStatus := diagnoseConnectivity(snapshot)
	sourceChecks, sourceIssues, sourceStatus := diagnoseSources(snapshot)
	spoolChecks, spoolIssues, spoolStatus := diagnoseSpool(snapshot)
	policyChecks, policyIssues := diagnosePolicy(snapshot)
	installChecks, installIssues := diagnoseInstall(snapshot)

	out.Checks = append(out.Checks, connectivityChecks...)
	out.Checks = append(out.Checks, sourceChecks...)
	out.Checks = append(out.Checks, spoolChecks...)
	out.Checks = append(out.Checks, policyChecks...)
	out.Checks = append(out.Checks, installChecks...)

	out.Issues = append(out.Issues, connectivityIssues...)
	out.Issues = append(out.Issues, sourceIssues...)
	out.Issues = append(out.Issues, spoolIssues...)
	out.Issues = append(out.Issues, policyIssues...)
	out.Issues = append(out.Issues, installIssues...)

	for _, message := range snapshot.Compatibility.Errors {
		out.Issues = append(out.Issues, model.AgentIssue{
			Code:     "compatibility_error",
			Severity: "error",
			Domain:   "runtime",
			Message:  message,
		})
	}
	for _, message := range snapshot.Compatibility.Warnings {
		out.Issues = append(out.Issues, model.AgentIssue{
			Code:     "compatibility_warning",
			Severity: "warn",
			Domain:   "runtime",
			Message:  message,
		})
	}

	sort.SliceStable(out.Issues, func(i, j int) bool {
		left := severityRank(out.Issues[i].Severity)
		right := severityRank(out.Issues[j].Severity)
		if left == right {
			if out.Issues[i].Domain == out.Issues[j].Domain {
				return out.Issues[i].Message < out.Issues[j].Message
			}
			return out.Issues[i].Domain < out.Issues[j].Domain
		}
		return left > right
	})

	for _, issue := range out.Issues {
		switch issue.Severity {
		case "error":
			out.Errors = append(out.Errors, issue)
		case "warn":
			out.Warnings = append(out.Warnings, issue)
		}
	}

	out.Summary.WarningCount = len(out.Warnings)
	out.Summary.FailureCount = len(out.Errors)
	out.Summary.TransportStatus = transportStatus
	out.Summary.TLSStatus = tlsStatus
	out.Summary.SourceStatus = sourceStatus
	out.Summary.SpoolStatus = spoolStatus
	out.Summary.TopIssues = topIssues(out.Issues)
	out.Summary.DoctorStatus = overallDoctorStatus(snapshot, out.Issues)

	return out
}

func diagnoseConnectivity(snapshot *diagnosticsSnapshot) ([]model.AgentDiagnosticsCheck, []model.AgentIssue, string, string) {
	if snapshot == nil {
		return []model.AgentDiagnosticsCheck{{
				CheckID:  "connectivity",
				Name:     "Edge connectivity",
				Status:   "unknown",
				Severity: "warn",
				Domain:   "transport",
				Message:  "Connectivity snapshot еще не поступал.",
			}, {
				CheckID:  "tls",
				Name:     "TLS transport",
				Status:   "unknown",
				Severity: "warn",
				Domain:   "tls",
				Message:  "TLS status недоступен.",
			}},
			nil,
			"unknown",
			"unknown"
	}

	transportStatus := connectivityTransportStatus(snapshot)
	tlsStatus := connectivityTLSStatus(snapshot)
	transportCheckStatus := "pass"
	transportSeverity := "info"
	switch transportStatus {
	case "blocked":
		transportCheckStatus = "fail"
		transportSeverity = "error"
	case "degraded":
		transportCheckStatus = "warn"
		transportSeverity = "warn"
	case "unknown":
		transportCheckStatus = "unknown"
		transportSeverity = "warn"
	}

	tlsCheckStatus := "pass"
	tlsSeverity := "info"
	switch tlsStatus {
	case "blocked", "degraded":
		tlsCheckStatus = "fail"
		tlsSeverity = "error"
	case "insecure":
		tlsCheckStatus = "warn"
		tlsSeverity = "warn"
	case "unknown":
		tlsCheckStatus = "unknown"
		tlsSeverity = "warn"
	}

	issues := make([]model.AgentIssue, 0, 6)
	if snapshot.TransportState.BlockedDelivery || snapshot.BlockedDelivery {
		issues = append(issues, model.AgentIssue{
			Code:     "transport_blocked",
			Severity: "error",
			Domain:   "transport",
			Message:  firstNonEmpty(ptrValue(snapshot.TransportState.BlockedReason), ptrValue(snapshot.BlockedReason), "Отправка батчей заблокирована транспортным слоем."),
		})
	}
	if snapshot.ConnectivityState.LastConnectError != nil {
		issues = append(issues, model.AgentIssue{
			Code:     "connect_error",
			Severity: "error",
			Domain:   "transport",
			Message:  *snapshot.ConnectivityState.LastConnectError,
		})
	}
	if snapshot.ConnectivityState.LastTLSError != nil {
		issues = append(issues, model.AgentIssue{
			Code:     "tls_error",
			Severity: "error",
			Domain:   "tls",
			Message:  *snapshot.ConnectivityState.LastTLSError,
		})
	}
	if snapshot.TransportState.ServerUnavailableForSec > 0 {
		issues = append(issues, model.AgentIssue{
			Code:     "server_unavailable",
			Severity: "warn",
			Domain:   "transport",
			Message:  fmt.Sprintf("Edge endpoint недоступен уже %d секунд.", snapshot.TransportState.ServerUnavailableForSec),
		})
	}
	if !snapshot.ConnectivityState.TLSEnabled || snapshot.Compatibility.InsecureTransport {
		issues = append(issues, model.AgentIssue{
			Code:     "tls_insecure",
			Severity: "warn",
			Domain:   "tls",
			Message:  "Соединение с edge-api не защищено TLS или отмечено как insecure.",
		})
	}
	if snapshot.ConnectivityState.MTLSEnabled &&
		(!snapshot.ConnectivityState.CaPathPresent || !snapshot.ConnectivityState.CertPathPresent || !snapshot.ConnectivityState.KeyPathPresent) {
		issues = append(issues, model.AgentIssue{
			Code:     "mtls_material_missing",
			Severity: "error",
			Domain:   "tls",
			Message:  "Для mTLS не хватает CA/cert/key material на хосте агента.",
		})
	}
	if snapshot.TransportState.LastErrorKind != nil && strings.TrimSpace(*snapshot.TransportState.LastErrorKind) != "" {
		domain := issueDomainFromErrorKind(*snapshot.TransportState.LastErrorKind)
		issues = append(issues, model.AgentIssue{
			Code:     "transport_error_kind",
			Severity: "warn",
			Domain:   domain,
			Message:  fmt.Sprintf("Последний transport error kind: %s.", *snapshot.TransportState.LastErrorKind),
		})
	}

	checks := []model.AgentDiagnosticsCheck{{
		CheckID:  "connectivity",
		Name:     "Edge connectivity",
		Status:   transportCheckStatus,
		Severity: transportSeverity,
		Domain:   "transport",
		Message:  connectivityHint(snapshot),
		Hint:     "Проверить reachability edge-api, firewall rules и состояние service/unit агента.",
	}, {
		CheckID:  "tls",
		Name:     "TLS transport",
		Status:   tlsCheckStatus,
		Severity: tlsSeverity,
		Domain:   "tls",
		Message:  firstNonEmpty(ptrValue(snapshot.ConnectivityState.LastTLSError), "TLS transport выглядит корректно."),
		Hint:     "Если TLS деградирован, проверить CA, cert/key, server_name и синхронизацию времени.",
	}}
	return checks, issues, transportStatus, tlsStatus
}

func diagnoseSources(snapshot *diagnosticsSnapshot) ([]model.AgentDiagnosticsCheck, []model.AgentIssue, string) {
	if snapshot == nil {
		return []model.AgentDiagnosticsCheck{{
			CheckID:  "sources",
			Name:     "Log sources",
			Status:   "unknown",
			Severity: "warn",
			Domain:   "source",
			Message:  "Состояние источников логов недоступно.",
		}}, nil, "unknown"
	}

	status := "healthy"
	checkStatus := "pass"
	checkSeverity := "info"
	issues := make([]model.AgentIssue, 0, len(snapshot.SourceStatuses)+len(snapshot.Compatibility.PermissionIssues)+len(snapshot.Compatibility.SourcePathIssues))

	if snapshot.ActiveSources == 0 && len(snapshot.SourceStatuses) == 0 {
		status = "idle"
		checkStatus = "warn"
		checkSeverity = "warn"
	}
	for _, item := range snapshot.Compatibility.PermissionIssues {
		issues = append(issues, model.AgentIssue{
			Code:     "permission_issue",
			Severity: "error",
			Domain:   "permissions",
			Message:  item,
		})
		status = "blocked"
		checkStatus = "fail"
		checkSeverity = "error"
	}
	for _, item := range snapshot.Compatibility.SourcePathIssues {
		issues = append(issues, model.AgentIssue{
			Code:     "source_path_issue",
			Severity: "error",
			Domain:   "source",
			Message:  item,
		})
		status = "blocked"
		checkStatus = "fail"
		checkSeverity = "error"
	}
	for _, source := range snapshot.SourceStatuses {
		sourceState := strings.ToLower(strings.TrimSpace(source.Status))
		switch {
		case strings.Contains(sourceState, "fail"), strings.Contains(sourceState, "error"), strings.Contains(sourceState, "missing"), strings.Contains(sourceState, "blocked"):
			status = "blocked"
			checkStatus = "fail"
			checkSeverity = "error"
			if source.LastError != nil {
				issues = append(issues, model.AgentIssue{
					Code:     "source_error",
					Severity: "error",
					Domain:   "source",
					Source:   firstNonEmpty(source.SourceID, source.Path),
					Message:  *source.LastError,
				})
			}
		case strings.Contains(sourceState, "warn"), strings.Contains(sourceState, "degraded"), strings.Contains(sourceState, "idle"):
			if status != "blocked" {
				status = "degraded"
			}
			if checkStatus != "fail" {
				checkStatus = "warn"
				checkSeverity = "warn"
			}
			if source.LastError != nil {
				issues = append(issues, model.AgentIssue{
					Code:     "source_warning",
					Severity: "warn",
					Domain:   "source",
					Source:   firstNonEmpty(source.SourceID, source.Path),
					Message:  *source.LastError,
				})
			}
		}
		if source.LivePendingBytes > 0 || source.DurablePendingBytes > 0 {
			if status == "healthy" {
				status = "degraded"
			}
			if checkStatus == "pass" {
				checkStatus = "warn"
				checkSeverity = "warn"
			}
			issues = append(issues, model.AgentIssue{
				Code:     "source_backlog",
				Severity: "warn",
				Domain:   "ingestion",
				Source:   firstNonEmpty(source.SourceID, source.Path),
				Message:  fmt.Sprintf("Источник `%s` отстает: pending live=%d durable=%d bytes.", firstNonEmpty(source.SourceID, source.Path), source.LivePendingBytes, source.DurablePendingBytes),
			})
		}
	}

	return []model.AgentDiagnosticsCheck{{
		CheckID:  "sources",
		Name:     "Log sources",
		Status:   checkStatus,
		Severity: checkSeverity,
		Domain:   "source",
		Message:  sourceHint(snapshot),
		Hint:     "Проверить source paths, права на чтение и факт появления новых записей в логах.",
	}}, issues, status
}

func diagnoseSpool(snapshot *diagnosticsSnapshot) ([]model.AgentDiagnosticsCheck, []model.AgentIssue, string) {
	if snapshot == nil {
		return []model.AgentDiagnosticsCheck{{
			CheckID:  "spool",
			Name:     "Local spool",
			Status:   "unknown",
			Severity: "warn",
			Domain:   "spool",
			Message:  "Состояние локального spool недоступно.",
		}}, nil, "unknown"
	}

	status := "healthy"
	checkStatus := "pass"
	checkSeverity := "info"
	message := "Spool не показывает накопленной очереди."
	issues := make([]model.AgentIssue, 0, 2)

	if !snapshot.SpoolEnabled {
		status = "disabled"
		message = "Локальный spool отключен."
	}
	if snapshot.SpooledBatches > 0 || snapshot.SpooledBytes > 0 {
		status = "degraded"
		checkStatus = "warn"
		checkSeverity = "warn"
		message = fmt.Sprintf("В spool накоплено %d батчей (%d bytes).", snapshot.SpooledBatches, snapshot.SpooledBytes)
		issues = append(issues, model.AgentIssue{
			Code:     "spool_backlog",
			Severity: "warn",
			Domain:   "spool",
			Message:  message,
		})
	}
	if snapshot.BlockedDelivery || snapshot.TransportState.BlockedDelivery {
		status = "blocked"
		checkStatus = "fail"
		checkSeverity = "error"
		message = firstNonEmpty(ptrValue(snapshot.BlockedReason), ptrValue(snapshot.TransportState.BlockedReason), "Spool перестал разгружаться из-за блокировки доставки.")
		issues = append(issues, model.AgentIssue{
			Code:     "spool_blocked",
			Severity: "error",
			Domain:   "spool",
			Message:  message,
		})
	}

	return []model.AgentDiagnosticsCheck{{
		CheckID:  "spool",
		Name:     "Local spool",
		Status:   checkStatus,
		Severity: checkSeverity,
		Domain:   "spool",
		Message:  message,
		Hint:     "Если backlog растет, проверить транспорт к edge-api и свободное место для spool.",
	}}, issues, status
}

func diagnosePolicy(snapshot *diagnosticsSnapshot) ([]model.AgentDiagnosticsCheck, []model.AgentIssue) {
	if snapshot == nil {
		return []model.AgentDiagnosticsCheck{{
			CheckID:  "policy",
			Name:     "Policy sync",
			Status:   "unknown",
			Severity: "warn",
			Domain:   "enrollment",
			Message:  "Сведения о policy sync отсутствуют.",
		}}, nil
	}

	checkStatus := "pass"
	checkSeverity := "info"
	message := "Policy ревизия загружена и применена."
	issues := make([]model.AgentIssue, 0, 1)

	if snapshot.PolicyState.LastPolicyError != nil {
		checkStatus = "fail"
		checkSeverity = "error"
		message = *snapshot.PolicyState.LastPolicyError
		issues = append(issues, model.AgentIssue{
			Code:     "policy_error",
			Severity: "error",
			Domain:   "enrollment",
			Message:  *snapshot.PolicyState.LastPolicyError,
		})
	} else if snapshot.PolicyState.CurrentPolicyRevision == nil || strings.TrimSpace(*snapshot.PolicyState.CurrentPolicyRevision) == "" {
		checkStatus = "warn"
		checkSeverity = "warn"
		message = "Policy revision пока не закреплена за агентом."
	}

	return []model.AgentDiagnosticsCheck{{
		CheckID:  "policy",
		Name:     "Policy sync",
		Status:   checkStatus,
		Severity: checkSeverity,
		Domain:   "enrollment",
		Message:  message,
		Hint:     "Проверить fetch/apply policy и binding на стороне control-plane.",
	}}, issues
}

func diagnoseInstall(snapshot *diagnosticsSnapshot) ([]model.AgentDiagnosticsCheck, []model.AgentIssue) {
	if snapshot == nil {
		return nil, nil
	}
	checkStatus := "pass"
	checkSeverity := "info"
	message := firstNonEmpty(diagnosticsInstallMode(snapshot), "Install mode не указан.")
	if strings.TrimSpace(message) != "" {
		message = fmt.Sprintf("Install mode: %s.", message)
	}
	issues := make([]model.AgentIssue, 0, len(snapshot.Install.Warnings))
	if len(snapshot.Install.Warnings) > 0 {
		checkStatus = "warn"
		checkSeverity = "warn"
		message = snapshot.Install.Warnings[0]
		for _, warning := range snapshot.Install.Warnings {
			issues = append(issues, model.AgentIssue{
				Code:     "install_warning",
				Severity: "warn",
				Domain:   "runtime",
				Message:  warning,
			})
		}
	}
	return []model.AgentDiagnosticsCheck{{
		CheckID:  "install",
		Name:     "Install mode",
		Status:   checkStatus,
		Severity: checkSeverity,
		Domain:   "runtime",
		Message:  message,
	}}, issues
}

func connectivityTransportStatus(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return "unknown"
	}
	if snapshot.BlockedDelivery || snapshot.TransportState.BlockedDelivery {
		return "blocked"
	}
	if snapshot.ConnectivityState.LastConnectError != nil ||
		snapshot.TransportState.ServerUnavailableForSec > 0 ||
		snapshot.ConsecutiveSendFailures > 0 {
		return "degraded"
	}
	return "healthy"
}

func connectivityTLSStatus(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return "unknown"
	}
	if snapshot.ConnectivityState.LastTLSError != nil {
		return "blocked"
	}
	if !snapshot.ConnectivityState.TLSEnabled || snapshot.Compatibility.InsecureTransport {
		return "insecure"
	}
	if snapshot.ConnectivityState.MTLSEnabled &&
		(!snapshot.ConnectivityState.CaPathPresent || !snapshot.ConnectivityState.CertPathPresent || !snapshot.ConnectivityState.KeyPathPresent) {
		return "degraded"
	}
	return "healthy"
}

func connectivityHint(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return "Агент еще не прислал connectivity snapshot."
	}
	switch {
	case snapshot.BlockedDelivery || snapshot.TransportState.BlockedDelivery:
		return "Доставка в edge-api заблокирована. Проверить reachability edge-api, сертификаты и журнал сервиса агента."
	case snapshot.ConnectivityState.LastTLSError != nil:
		return "TLS handshake не проходит. Проверить CA, cert/key, server_name и системное время на хосте."
	case snapshot.ConnectivityState.LastConnectError != nil:
		return "Агент не может установить соединение с edge-api. Проверить сеть, DNS, firewall и статус unit."
	case snapshot.TransportState.ServerUnavailableForSec > 0:
		return fmt.Sprintf("Edge endpoint недоступен уже %d секунд. Проверить публичный edge-api и сетевой маршрут.", snapshot.TransportState.ServerUnavailableForSec)
	case !snapshot.ConnectivityState.TLSEnabled:
		return "Агент подключается без TLS. Это рабочий, но деградированный режим для production."
	default:
		return "Последнее подключение к edge-api выглядит стабильным."
	}
}

func sourceHint(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return "Snapshot по источникам логов отсутствует."
	}
	switch {
	case len(snapshot.Compatibility.PermissionIssues) > 0:
		return "Недостаточно прав на чтение одного или нескольких log source."
	case len(snapshot.Compatibility.SourcePathIssues) > 0:
		return "Один или несколько путей источников недоступны или отсутствуют."
	case snapshot.ActiveSources == 0 && len(snapshot.SourceStatuses) == 0:
		return "У агента сейчас нет активных log source."
	default:
		for _, source := range snapshot.SourceStatuses {
			if source.LastError != nil {
				return "Один или несколько log source возвращают ошибки чтения."
			}
			if source.LivePendingBytes > 0 || source.DurablePendingBytes > 0 {
				return "Источник логов читает с отставанием; стоит проверить backlog и активность файла."
			}
		}
	}
	return "Источники логов выглядят рабочими."
}

func diagnosticsInstallMode(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return ""
	}
	return strings.TrimSpace(snapshot.Install.ResolvedMode)
}

func issueDomainFromErrorKind(kind string) string {
	normalized := strings.ToLower(strings.TrimSpace(kind))
	switch {
	case normalized == "":
		return "unknown"
	case strings.Contains(normalized, "tls"), strings.Contains(normalized, "cert"):
		return "tls"
	case strings.Contains(normalized, "permission"), strings.Contains(normalized, "unauthorized"), strings.Contains(normalized, "forbidden"):
		return "permissions"
	case strings.Contains(normalized, "network"), strings.Contains(normalized, "connect"), strings.Contains(normalized, "timeout"), strings.Contains(normalized, "transient"):
		return "transport"
	case strings.Contains(normalized, "source"), strings.Contains(normalized, "path"):
		return "source"
	case strings.Contains(normalized, "spool"), strings.Contains(normalized, "queue"):
		return "spool"
	case strings.Contains(normalized, "ingest"), strings.Contains(normalized, "serial"), strings.Contains(normalized, "payload"):
		return "ingestion"
	case strings.Contains(normalized, "enroll"), strings.Contains(normalized, "identity"), strings.Contains(normalized, "policy"):
		return "enrollment"
	case strings.Contains(normalized, "runtime"):
		return "runtime"
	default:
		return "unknown"
	}
}

func overallDoctorStatus(snapshot *diagnosticsSnapshot, issues []model.AgentIssue) string {
	if snapshot == nil {
		return "unknown"
	}
	hasWarn := false
	for _, issue := range issues {
		if issue.Severity == "error" {
			return "fail"
		}
		if issue.Severity == "warn" {
			hasWarn = true
		}
	}
	if snapshot.DegradedMode || hasWarn {
		return "warn"
	}
	if strings.TrimSpace(snapshot.RuntimeStatus) == "" {
		return "unknown"
	}
	return "pass"
}

func topIssues(issues []model.AgentIssue) []model.AgentIssue {
	if len(issues) == 0 {
		return []model.AgentIssue{}
	}
	limit := len(issues)
	if limit > 5 {
		limit = 5
	}
	out := make([]model.AgentIssue, 0, limit)
	out = append(out, issues[:limit]...)
	return out
}

func filterIssuesByDomains(issues []model.AgentIssue, domains ...string) []model.AgentIssue {
	if len(issues) == 0 || len(domains) == 0 {
		return []model.AgentIssue{}
	}
	allowed := make(map[string]struct{}, len(domains))
	for _, domain := range domains {
		allowed[strings.TrimSpace(domain)] = struct{}{}
	}
	out := make([]model.AgentIssue, 0, len(issues))
	for _, issue := range issues {
		if _, ok := allowed[issue.Domain]; ok {
			out = append(out, issue)
		}
	}
	return out
}

func hasIssueDomain(issues []model.AgentIssue, domains ...string) bool {
	return len(filterIssuesByDomains(issues, domains...)) > 0
}

func severityRank(severity string) int {
	switch strings.ToLower(strings.TrimSpace(severity)) {
	case "error":
		return 3
	case "warn":
		return 2
	case "info":
		return 1
	default:
		return 0
	}
}

func determinePrimaryFailureDomain(status model.HostAgentStatusView, presentation diagnosticsPresentation) string {
	phase := strings.ToLower(strings.TrimSpace(status.DeploymentCurrentPhase))
	switch {
	case status.DeploymentStatus == "failed", strings.Contains(phase, "ansible.failed"), strings.Contains(phase, "artifact"), strings.Contains(phase, "rollback"):
		return "deployment"
	case status.EnrollmentStatus != "enrolled":
		return "enrollment"
	case hasIssueDomain(presentation.Issues, "tls"), status.TLSStatus == "blocked", status.TLSStatus == "degraded", status.TLSStatus == "insecure":
		return "tls"
	case status.HeartbeatStatus == "stale", status.HeartbeatStatus == "missing", hasIssueDomain(presentation.Issues, "transport"):
		return "network"
	case hasIssueDomain(presentation.Issues, "permissions", "source", "spool", "ingestion"), status.SourceStatus == "blocked", status.SourceStatus == "degraded":
		return "ingestion"
	case status.DoctorStatus == "fail", status.DoctorStatus == "warn", hasIssueDomain(presentation.Issues, "runtime"):
		return "runtime"
	default:
		return "unknown"
	}
}

func buildHint(status model.HostAgentStatusView, presentation diagnosticsPresentation) (string, string) {
	phase := strings.ToLower(strings.TrimSpace(status.DeploymentCurrentPhase))
	switch {
	case status.DeploymentStatus == "failed" && (strings.Contains(phase, "artifact") || strings.Contains(phase, "image")):
		return "Контейнерный runtime не смог скачать или разрешить артефакт агента. Проверить Docker/Podman, registry access и image reference.", "Открыть deployment timeline, проверить stderr ansible-runner и artifact reference."
	case status.DeploymentStatus == "failed":
		return "Развертывание агента не завершилось успешно. Ошибка пока выглядит как проблема deployment-пайплайна, а не runtime.", "Проверить deployment timeline и последний ansible target result для этого host."
	case status.EnrollmentStatus != "enrolled" && status.LastSuccessfulDeployAt != "":
		return "Агент был установлен, но не завершил enrollment через edge-api. Проверить bootstrap token, доступность edge-api и журналы сервиса агента.", "Сверить bootstrap config на host и открыть `journalctl -u` для unit агента."
	case status.HeartbeatStatus == "stale" && status.LastSuccessfulDeployAt != "":
		return "Агент был установлен, но перестал отправлять heartbeat. Проверить доступность edge-api и статус systemd unit на хосте.", "Проверить `systemctl status`, сетевой маршрут до edge-api и последние transport ошибки."
	case hasIssueDomain(presentation.Issues, "permissions"):
		return "Недостаточно прав на чтение одного или нескольких log source.", "Выдать агенту доступ к нужным файлам/каталогам и перепроверить diagnostics snapshot."
	case status.PrimaryFailureDomain == "tls":
		return "Проблема выглядит как TLS/mTLS ошибка между агентом и edge-api.", "Проверить CA, cert/key, server_name и синхронизацию времени на host."
	case status.PrimaryFailureDomain == "network":
		return "Агент не удерживает стабильную связь с edge-api.", "Проверить DNS, firewall, публичный адрес edge-api и состояние сервисного процесса агента."
	case status.PrimaryFailureDomain == "ingestion":
		return "Агент подключен, но ingest-path деградирован: нет логов, backlog по source или проблемы со spool.", "Проверить source paths, file activity и backlog в diagnostics detail."
	case status.PrimaryFailureDomain == "runtime":
		return "Runtime агента сообщает деградацию или внутренние ошибки.", "Открыть diagnostics detail и разобрать top issues / runtime errors."
	default:
		return "Снимок состояния собран, но явная доменная причина пока не выделена.", "Открыть diagnostics snapshot и последние deployment steps."
	}
}

func severityFromStatus(status model.HostAgentStatusView) string {
	switch {
	case status.DeploymentStatus == "failed", status.DoctorStatus == "fail", status.HeartbeatStatus == "stale":
		return "error"
	case status.EnrollmentStatus != "enrolled", status.DoctorStatus == "warn", status.HeartbeatStatus == "missing", status.PrimaryFailureDomain != "unknown":
		return "warn"
	default:
		return "info"
	}
}
