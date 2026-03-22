import type { Locale } from "@/src/shared/config";

const valueLabels = {
  en: {
    active: "Active",
    inactive: "Inactive",
    enabled: "Enabled",
    disabled: "Disabled",
    paused: "Paused",
    degraded: "Degraded",
    online: "Online",
    healthy: "Healthy",
    ready: "Ready",
    unavailable: "Unavailable",
    unknown: "Unknown",
    open: "Open",
    closed: "Closed",
    resolved: "Resolved",
    delivered: "Delivered",
    queued: "Queued",
    blocked: "Blocked",
    "not-configured": "Not configured",
    success: "Success",
    succeeded: "Succeeded",
    failed: "Failed",
    running: "Running",
    pending: "Pending",
    cancelled: "Cancelled",
    canceled: "Cancelled",
    warning: "Warning",
    critical: "Critical",
    high: "High",
    medium: "Medium",
    low: "Low",
    info: "Info",
    debug: "Debug",
    fatal: "Fatal",
    error: "Error",
    watching: "Watching",
    default: "Default",
    pull: "Pull",
    start: "Start",
    health: "Health",
    rollback: "Rollback",
    retry: "Retry",
    light: "Light",
    heavy: "Heavy",
    medium_mode: "Medium",
    all: "All",
    unassigned: "Unassigned",
  },
  ru: {
    active: "Активно",
    inactive: "Неактивно",
    enabled: "Включено",
    disabled: "Выключено",
    paused: "Пауза",
    degraded: "Деградация",
    online: "В сети",
    healthy: "Исправно",
    ready: "Готово",
    unavailable: "Недоступно",
    unknown: "Неизвестно",
    open: "Открыто",
    closed: "Закрыто",
    resolved: "Решено",
    delivered: "Доставлено",
    queued: "В очереди",
    blocked: "Заблокировано",
    "not-configured": "Не настроено",
    success: "Успех",
    succeeded: "Завершено",
    failed: "Ошибка",
    running: "Выполняется",
    pending: "Ожидает",
    cancelled: "Отменено",
    canceled: "Отменено",
    warning: "Предупреждение",
    critical: "Критично",
    high: "Высоко",
    medium: "Средне",
    low: "Низко",
    info: "Инфо",
    debug: "Отладка",
    fatal: "Фатально",
    error: "Ошибка",
    watching: "Под наблюдением",
    default: "По умолчанию",
    pull: "Загрузка",
    start: "Запуск",
    health: "Проверка",
    rollback: "Откат",
    retry: "Повтор",
    light: "Легкий",
    heavy: "Глубокий",
    medium_mode: "Средний",
    all: "Все",
    unassigned: "Не назначено",
  },
} as const;

const siteCopy = {
  en: {
    common: {
      na: "n/a",
      notSet: "not set",
      notRun: "Not run",
      open: "Open",
      clear: "Clear",
      apply: "Apply",
      retry: "Retry",
      previous: "Previous",
      next: "Next",
      unavailable: "Unavailable",
    },
    navigation: {
      overview: "Overview",
      infrastructure: "Infrastructure",
      security: "Security",
      operations: "Operations",
      integrations: "Integrations",
      audit: "Audit",
      profile: "Profile",
      breadcrumb: "Breadcrumb",
    },
    langSwitch: {
      label: "Language",
      switchLabel: "Switch language",
      ariaLabel: "Language switch",
      russian: "Russian",
      english: "English",
    },
    toast: {
      dismiss: "Dismiss notification",
    },
    runtimeState: {
      loadingData: "Loading data...",
      requestFailed: "Request failed",
    },
    operationsUi: {
      backendUnexpected: "The backend returned an unexpected error.",
      noLabels: "No labels",
      noItems: "No items",
      noJson: "No JSON payload available.",
      noParams: "No params",
      noDataAvailable: "No data available.",
      noHistogramData: "No histogram data available.",
      statusLabel: "Status",
      requestIdLabel: "Request ID",
      subjectLabel: "Subject",
      retry: "Retry",
      previous: "Previous",
      next: "Next",
    },
    datePicker: {
      locale: "en-US",
      weekDays: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
      months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
      ],
      time: "Time",
      hours: "Hours",
      minutes: "Minutes",
      selectedPrefix: "Selected:",
      noDateSelected: "No date selected",
      clear: "Clear",
      apply: "Apply",
    },
    workbench: {
      security: {
        summary: {
          openAlerts: {
            label: "Open alerts",
            helperText:
              "Current runtime alert pressure derived from alert instances.",
          },
          healthyAgents: {
            label: "Healthy agents",
            helperText:
              "Coverage ratio based on the agent registry health states.",
          },
          activePolicies: {
            label: "Active policies",
            helperText:
              "Policy posture is inferred from active policy metadata in the runtime API.",
          },
          rolloutRisk: {
            label: "Failing rollouts",
            helperText:
              "Rollback and failed deployment jobs that may amplify operator load.",
          },
        },
        findings: {
          agentsCoverage: {
            title: "Agent coverage requires attention",
            summary: (count: string) =>
              `${count} agent(s) are not healthy or online.`,
            impact:
              "Alerting and posture calculations may miss signals from affected hosts while these agents remain degraded.",
            recommendedAction:
              "Inspect affected agents, confirm last-seen timestamps, and restore telemetry before broad rollouts.",
            relatedRouteLabel: "Open agents",
          },
          openAlerts: {
            title: "Open operator alerts are accumulating",
            summary: (count: string) =>
              `${count} alert(s) still require operator follow-up.`,
            impact:
              "The console already shows active alert pressure, which increases the chance of delayed acknowledgements.",
            recommendedAction:
              "Review correlated anomalies and delivery routing, then acknowledge or resolve stale alerts.",
            relatedRouteLabel: "Open alerts",
          },
          inactivePolicies: {
            title: "Inactive policies detected",
            summary: (count: string) =>
              `${count} policy profile(s) are inactive.`,
            impact:
              "Inactive policy revisions reduce coverage for drift detection and deployment guardrails.",
            recommendedAction:
              "Confirm whether inactive policies are intentional, and reactivate or archive the stale entries.",
            relatedRouteLabel: "Open policies",
          },
          deploymentInstability: {
            title: "Recent rollout instability",
            summary: (count: string) =>
              `${count} deployment job(s) show failed or rollback states.`,
            impact:
              "Rollback-heavy deployment activity often precedes noisy alerts and degraded host trust.",
            recommendedAction:
              "Inspect rollout phases, confirm image health, and avoid widening the blast radius until the failing jobs stabilize.",
            relatedRouteLabel: "Open deployments",
          },
          auditBurst: {
            title: "High control-plane change activity",
            summary: (count: string) =>
              `${count} recent audit events were observed in the runtime log.`,
            impact:
              "Dense change windows make root-cause analysis harder when alerts and anomalies begin stacking.",
            evidence:
              "Recent audit volume exceeded the dashboard watch threshold.",
            recommendedAction:
              "Confirm whether the current change window is expected and coordinate alert ownership before additional mutations.",
            relatedRouteLabel: "Open audit",
          },
          deploymentPrefix: "Deployment",
          unknownStatus: "unknown",
          unscopedTarget: "unscoped target",
        },
      },
      anomalies: {
        modes: {
          light: {
            label: "Light",
            subtitle: "Fast operator scan",
            description:
              "Shows the noisiest recent anomalies with a short correlation window. Good for triage and demos.",
            explanation:
              "Fast triage mode keeps the immediate alert correlation only.",
          },
          medium: {
            label: "Medium",
            subtitle: "Balanced signal density",
            description:
              "Balances coverage and readability by keeping the latest anomalies plus correlated open alerts.",
            explanation:
              "Balanced mode keeps the anomaly plus nearby open alerts for operator review.",
          },
          heavy: {
            label: "Heavy",
            subtitle: "Deep correlation sweep",
            description:
              "Keeps a longer timeline and fuller alert context. This is still a frontend operator lens until backend-side anomaly mode contracts arrive.",
            explanation:
              "Heavy mode keeps a wider operator correlation window. Backend-side anomaly mode control is still pending, so this is a frontend lens.",
          },
        },
        unknownHost: "unknown host",
        unknownService: "unknown service",
        correlatedAlertsSuffix: (count: number) =>
          `with ${count} correlated open alert(s).`,
        alertStillOpenSuffix: "is still open.",
      },
      alerts: {
        explanationWithAnomaly: (
          title: string,
          host: string,
          deliveries: number
        ) =>
          `Alert ${title} is backed by a log anomaly on ${host} and currently routes through ${deliveries} delivery path(s).`,
        explanationWithoutAnomaly: (title: string, ruleName: string) =>
          `Alert ${title} is active under ${ruleName} and is using frontend delivery projections until a dedicated delivery-status backend contract is published.`,
        rule: "Rule",
        severity: "Severity",
        status: "Status",
        host: "Host",
        service: "Service",
        fingerprint: "Fingerprint",
      },
      deploymentImage: {
        noArtifactLabel: "No artifact metadata yet",
        noArtifactDescription:
          "The runtime API has not returned a deployment artifact for the selected job.",
        imageFallbackName: "image",
        noSourceUri: "No source URI returned",
        rollingInstall: "Rolling image install",
        singleInstall: "Single-target image install",
        unknown: "unknown",
        phases: {
          pull: {
            label: "Pull",
            detail:
              "Pulls the deployment image or package onto the target host.",
          },
          start: {
            label: "Start",
            detail:
              "Starts the workload or service with the requested image revision.",
          },
          health: {
            label: "Health",
            detail:
              "Verifies probes and post-start runtime health before promotion.",
          },
          rollback: {
            label: "Rollback",
            detail:
              "Prepared rollback state when any target fails health or startup.",
          },
        },
      },
    },
  },
  ru: {
    common: {
      na: "н/д",
      notSet: "не задано",
      notRun: "Не запускалось",
      open: "Открыть",
      clear: "Очистить",
      apply: "Применить",
      retry: "Повторить",
      previous: "Назад",
      next: "Далее",
      unavailable: "Недоступно",
    },
    navigation: {
      overview: "обзор",
      infrastructure: "инфраструктура",
      security: "безопасность",
      operations: "операции/логи",
      integrations: "интеграции",
      audit: "аудит",
      profile: "профиль",
      breadcrumb: "Хлебные крошки",
    },
    langSwitch: {
      label: "Язык",
      switchLabel: "Переключить язык",
      ariaLabel: "Переключатель языка",
      russian: "Русский",
      english: "English",
    },
    toast: {
      dismiss: "Закрыть уведомление",
    },
    runtimeState: {
      loadingData: "Загрузка данных...",
      requestFailed: "Не удалось выполнить запрос",
    },
    operationsUi: {
      backendUnexpected: "Бэкенд вернул неожиданную ошибку.",
      noLabels: "Нет меток",
      noItems: "Нет элементов",
      noJson: "JSON-пейлоад недоступен.",
      noParams: "Нет параметров",
      noDataAvailable: "Данных нет.",
      noHistogramData: "Нет данных для гистограммы.",
      statusLabel: "Статус",
      requestIdLabel: "Request ID",
      subjectLabel: "Тема",
      retry: "Повторить",
      previous: "Назад",
      next: "Далее",
    },
    datePicker: {
      locale: "ru-RU",
      weekDays: ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
      months: [
        "Январь",
        "Февраль",
        "Март",
        "Апрель",
        "Май",
        "Июнь",
        "Июль",
        "Август",
        "Сентябрь",
        "Октябрь",
        "Ноябрь",
        "Декабрь",
      ],
      time: "Время",
      hours: "Часы",
      minutes: "Минуты",
      selectedPrefix: "Выбрано:",
      noDateSelected: "Дата не выбрана",
      clear: "Очистить",
      apply: "Применить",
    },
    workbench: {
      security: {
        summary: {
          openAlerts: {
            label: "Открытые алерты",
            helperText:
              "Текущее давление по алертам, собранное из runtime-инстансов.",
          },
          healthyAgents: {
            label: "Исправные агенты",
            helperText:
              "Соотношение покрытия по состояниям реестра агентов.",
          },
          activePolicies: {
            label: "Активные политики",
            helperText:
              "Состояние политик выводится из активных записей в runtime API.",
          },
          rolloutRisk: {
            label: "Проблемные выкаты",
            helperText:
              "Неудачные и rollback-задачи, которые увеличивают нагрузку на операторов.",
          },
        },
        findings: {
          agentsCoverage: {
            title: "Покрытие агентами требует внимания",
            summary: (count: string) =>
              `${count} агент(ов) не в состоянии healthy или online.`,
            impact:
              "Алерты и расчеты posture могут пропускать сигналы с затронутых хостов, пока эти агенты деградированы.",
            recommendedAction:
              "Проверьте проблемные агенты, last-seen и восстановите телеметрию до расширения раскатки.",
            relatedRouteLabel: "Открыть агентов",
          },
          openAlerts: {
            title: "Накапливаются открытые алерты",
            summary: (count: string) =>
              `${count} алерт(ов) все еще ждут реакции оператора.`,
            impact:
              "В консоли уже есть активное давление по алертам, что повышает риск позднего подтверждения.",
            recommendedAction:
              "Проверьте связанные аномалии и маршрутизацию доставки, затем подтвердите или закройте старые алерты.",
            relatedRouteLabel: "Открыть алерты",
          },
          inactivePolicies: {
            title: "Обнаружены неактивные политики",
            summary: (count: string) =>
              `${count} профилей политик находятся в неактивном состоянии.`,
            impact:
              "Неактивные ревизии политик снижают покрытие drift detection и guardrails для раскаток.",
            recommendedAction:
              "Проверьте, намеренно ли политики отключены, и активируйте или архивируйте устаревшие записи.",
            relatedRouteLabel: "Открыть политики",
          },
          deploymentInstability: {
            title: "Нестабильность последних раскаток",
            summary: (count: string) =>
              `${count} задач deployment имеют failed или rollback-состояния.`,
            impact:
              "Частые rollback и ошибки раскатки часто предшествуют шумным алертам и снижению доверия к хостам.",
            recommendedAction:
              "Проверьте фазы раскатки, здоровье образа и не расширяйте blast radius, пока проблемные задачи не стабилизируются.",
            relatedRouteLabel: "Открыть раскатки",
          },
          auditBurst: {
            title: "Высокая активность изменений control-plane",
            summary: (count: string) =>
              `В runtime-логе замечено ${count} недавних audit-событий.`,
            impact:
              "Плотные окна изменений усложняют root-cause анализ, когда начинают накапливаться алерты и аномалии.",
            evidence:
              "Объем недавних audit-событий превысил порог наблюдения на дашборде.",
            recommendedAction:
              "Подтвердите, что текущее окно изменений ожидаемо, и заранее распределите владельцев алертов.",
            relatedRouteLabel: "Открыть аудит",
          },
          deploymentPrefix: "Раскатка",
          unknownStatus: "неизвестно",
          unscopedTarget: "неопределенная цель",
        },
      },
      anomalies: {
        modes: {
          light: {
            label: "Легкий",
            subtitle: "Быстрый операторский просмотр",
            description:
              "Показывает самые шумные недавние аномалии с коротким окном корреляции. Подходит для triage и демо.",
            explanation:
              "Режим быстрого triage оставляет только ближайшую корреляцию с алертами.",
          },
          medium: {
            label: "Средний",
            subtitle: "Сбалансированная плотность сигналов",
            description:
              "Сохраняет баланс между покрытием и читаемостью: последние аномалии плюс связанные открытые алерты.",
            explanation:
              "Сбалансированный режим показывает аномалию и соседние открытые алерты для операторского разбора.",
          },
          heavy: {
            label: "Глубокий",
            subtitle: "Глубокая корреляция",
            description:
              "Держит более длинную временную линию и более полный контекст алертов. Это все еще фронтенд-линза, пока backend-контракты режимов не готовы.",
            explanation:
              "Глубокий режим оставляет более широкое окно корреляции. Управление режимом на стороне backend пока еще не готово, поэтому это фронтенд-представление.",
          },
        },
        unknownHost: "неизвестный хост",
        unknownService: "неизвестный сервис",
        correlatedAlertsSuffix: (count: number) =>
          `${count} связанных открытых алерт(ов).`,
        alertStillOpenSuffix: "алерт все еще открыт.",
      },
      alerts: {
        explanationWithAnomaly: (
          title: string,
          host: string,
          deliveries: number
        ) =>
          `Алерт ${title} подкреплен логовой аномалией на ${host} и сейчас проходит через ${deliveries} маршрут(ов) доставки.`,
        explanationWithoutAnomaly: (title: string, ruleName: string) =>
          `Алерт ${title} активен по правилу ${ruleName} и использует фронтенд-проекции доставки, пока не появится отдельный backend-контракт статусов доставки.`,
        rule: "Правило",
        severity: "Severity",
        status: "Статус",
        host: "Хост",
        service: "Сервис",
        fingerprint: "Fingerprint",
      },
      deploymentImage: {
        noArtifactLabel: "Метаданные артефакта еще не получены",
        noArtifactDescription:
          "Runtime API еще не вернул артефакт раскатки для выбранной задачи.",
        imageFallbackName: "образ",
        noSourceUri: "Source URI не возвращен",
        rollingInstall: "Пошаговая установка образа",
        singleInstall: "Установка образа на один таргет",
        unknown: "неизвестно",
        phases: {
          pull: {
            label: "Загрузка",
            detail:
              "Загружает deployment image или пакет на целевой хост.",
          },
          start: {
            label: "Запуск",
            detail:
              "Запускает workload или сервис с запрошенной ревизией образа.",
          },
          health: {
            label: "Проверка",
            detail:
              "Проверяет пробы и здоровье runtime после старта перед продвижением.",
          },
          rollback: {
            label: "Откат",
            detail:
              "Подготавливает состояние для отката, если любой таргет не проходит старт или health-check.",
          },
        },
      },
    },
  },
} as const;

export function getSiteCopy(locale: Locale) {
  return siteCopy[locale];
}

export function translateValueLabel(
  value: string | number | null | undefined,
  locale: Locale
) {
  if (value == null || value === "") {
    return getSiteCopy(locale).common.na;
  }

  const normalized = String(value).trim().toLowerCase().replace(/\s+/g, "-");

  if (normalized === "medium") {
    return valueLabels[locale].medium;
  }

  return (
    valueLabels[locale][normalized as keyof (typeof valueLabels)[Locale]] ??
    String(value)
  );
}

export function translateToneLabel(
  value: "danger" | "warning" | "success" | "default",
  locale: Locale
) {
  return translateValueLabel(value, locale);
}
