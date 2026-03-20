const specVersionEl = document.getElementById('spec-version');
const specTitleEl = document.getElementById('spec-title');
const specDescriptionEl = document.getElementById('spec-description');
const serverListEl = document.getElementById('server-list');
const operationsEl = document.getElementById('operations');
const tagNavEl = document.getElementById('tag-nav');
const filterInput = document.getElementById('endpoint-filter');
const template = document.getElementById('operation-template');

const METHOD_ORDER = ['get', 'post', 'put', 'patch', 'delete'];

fetch('/openapi.json')
  .then((response) => {
    if (!response.ok) {
      throw new Error(`Unable to load spec: ${response.status}`);
    }
    return response.json();
  })
  .then((spec) => renderSpec(spec))
  .catch((error) => {
    operationsEl.innerHTML = `<div class="empty-state">${error.message}</div>`;
    specVersionEl.textContent = 'Spec load failed';
  });

function renderSpec(spec) {
  specVersionEl.textContent = `Version ${spec.info?.version ?? 'n/a'} · OpenAPI ${spec.openapi}`;
  specTitleEl.textContent = spec.info?.title ?? 'OpenAPI';
  specDescriptionEl.textContent = spec.info?.description ?? '';

  serverListEl.replaceChildren(...(spec.servers ?? []).map((server) => {
    const badge = document.createElement('span');
    badge.className = 'server-badge';
    badge.textContent = `${server.url}${server.description ? ` · ${server.description}` : ''}`;
    return badge;
  }));

  const operations = collectOperations(spec);
  const tags = orderTags(spec.tags ?? [], operations);
  renderTagNav(tags);
  renderOperations(spec, tags, operations);

  filterInput.addEventListener('input', () => renderOperations(spec, tags, operations));
}

function collectOperations(spec) {
  const result = [];
  for (const [path, pathItem] of Object.entries(spec.paths ?? {})) {
    for (const method of METHOD_ORDER) {
      const operation = pathItem?.[method];
      if (!operation) continue;
      result.push({ path, method, operation, pathItem });
    }
  }
  return result;
}

function orderTags(tagDefinitions, operations) {
  const tagMap = new Map();
  tagDefinitions.forEach((tag) => tagMap.set(tag.name, tag));
  operations.forEach(({ operation }) => {
    const tagName = operation.tags?.[0] ?? 'misc';
    if (!tagMap.has(tagName)) {
      tagMap.set(tagName, { name: tagName, description: '' });
    }
  });
  return Array.from(tagMap.values());
}

function renderTagNav(tags) {
  tagNavEl.replaceChildren(...tags.map((tag) => {
    const link = document.createElement('a');
    link.className = 'tag-link';
    link.href = `#tag-${slug(tag.name)}`;
    link.textContent = tag.name;
    return link;
  }));
}

function renderOperations(spec, tags, operations) {
  const query = filterInput.value.trim().toLowerCase();
  operationsEl.innerHTML = '';

  const filtered = operations.filter(({ path, method, operation }) => {
    if (!query) return true;
    const haystack = [path, method, operation.summary, operation.description, ...(operation.tags ?? [])]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    return haystack.includes(query);
  });

  if (!filtered.length) {
    operationsEl.innerHTML = '<div class="empty-state">No endpoints match the current filter.</div>';
    return;
  }

  tags.forEach((tag) => {
    const tagOperations = filtered.filter(({ operation }) => (operation.tags?.[0] ?? 'misc') === tag.name);
    if (!tagOperations.length) return;

    const section = document.createElement('section');
    section.className = 'tag-section';
    section.id = `tag-${slug(tag.name)}`;

    const title = document.createElement('h3');
    title.textContent = tag.name;
    section.appendChild(title);

    if (tag.description) {
      const description = document.createElement('p');
      description.className = 'description';
      description.textContent = tag.description;
      section.appendChild(description);
    }

    tagOperations.forEach((entry) => section.appendChild(renderOperationCard(spec, entry)));
    operationsEl.appendChild(section);
  });
}

function renderOperationCard(spec, entry) {
  const { path, method, operation, pathItem } = entry;
  const node = template.content.firstElementChild.cloneNode(true);
  const methodEl = node.querySelector('.method');
  methodEl.textContent = method.toUpperCase();
  methodEl.classList.add(method);
  node.querySelector('.path').textContent = path;
  node.querySelector('.summary').textContent = operation.summary ?? '';
  node.querySelector('.description').textContent = operation.description ?? operation.summary ?? '—';

  const chips = node.querySelector('.chips');
  const chipValues = [operation.operationId, ...(operation.tags ?? [])].filter(Boolean);
  chips.replaceChildren(...chipValues.map((value) => makeChip(value)));

  const parameters = [...(pathItem.parameters ?? []), ...(operation.parameters ?? [])].map((parameter) => deref(spec, parameter));
  node.querySelector('.parameters').replaceChildren(renderParameters(parameters));

  const requestBody = operation.requestBody ? deref(spec, operation.requestBody) : null;
  node.querySelector('.request-body').replaceChildren(renderRequestBody(spec, requestBody));

  const responses = node.querySelector('.responses');
  responses.replaceChildren(renderResponses(spec, operation.responses ?? {}));

  return node;
}

function renderParameters(parameters) {
  if (!parameters.length) {
    return wrapEmpty('No parameters.');
  }
  const list = document.createElement('div');
  list.className = 'kv-list';
  parameters.forEach((parameter) => {
    const item = document.createElement('div');
    item.className = 'kv-item';
    const schema = parameter.schema ? formatSchema(parameter.schema) : 'unknown';
    item.innerHTML = `
      <strong><code>${parameter.name}</code></strong> <span class="muted">(${parameter.in}${parameter.required ? ', required' : ''})</span>
      <div class="schema-title">${schema}</div>
      <div class="description">${parameter.description ?? '—'}</div>
    `;
    list.appendChild(item);
  });
  return list;
}

function renderRequestBody(spec, requestBody) {
  if (!requestBody) return wrapEmpty('No request body.');
  const content = requestBody.content ?? {};
  const list = document.createElement('div');
  list.className = 'kv-list';
  Object.entries(content).forEach(([contentType, mediaType]) => {
    const item = document.createElement('div');
    item.className = 'kv-item';
    const schema = mediaType.schema ? formatSchema(deref(spec, mediaType.schema)) : 'unknown';
    item.innerHTML = `<strong>${contentType}</strong><div class="schema-title">${schema}</div>`;
    if (mediaType.example !== undefined) {
      item.appendChild(renderJSON(mediaType.example));
    }
    list.appendChild(item);
  });
  return list;
}

function renderResponses(spec, responses) {
  const wrapper = document.createElement('div');
  wrapper.className = 'response-list';
  Object.entries(responses).forEach(([status, response]) => {
    const resolved = deref(spec, response);
    const item = document.createElement('div');
    item.className = 'response-item';
    const statusClass = status.startsWith('2') ? 'status-2xx' : status.startsWith('4') ? 'status-4xx' : 'status-5xx';
    item.innerHTML = `<div><span class="status-pill ${statusClass}">${status}</span>${resolved.description ?? ''}</div>`;
    const mediaTypes = resolved.content ? Object.entries(resolved.content) : [];
    mediaTypes.forEach(([contentType, mediaType]) => {
      const block = document.createElement('div');
      block.innerHTML = `<div class="schema-title">${contentType} · ${formatSchema(deref(spec, mediaType.schema ?? {}))}</div>`;
      if (mediaType.example !== undefined) {
        block.appendChild(renderJSON(mediaType.example));
      }
      item.appendChild(block);
    });
    wrapper.appendChild(item);
  });
  return wrapper;
}

function renderJSON(value) {
  const pre = document.createElement('pre');
  const code = document.createElement('code');
  code.textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
  pre.appendChild(code);
  return pre;
}

function wrapEmpty(text) {
  const div = document.createElement('div');
  div.className = 'empty';
  div.textContent = text;
  return div;
}

function makeChip(label) {
  const chip = document.createElement('span');
  chip.className = 'chip';
  chip.textContent = label;
  return chip;
}

function formatSchema(schema) {
  if (!schema || Object.keys(schema).length === 0) return 'schema: n/a';
  if (schema.$ref) return schema.$ref.replace('#/components/schemas/', 'schema: ');
  if (schema.type === 'array') return `array<${formatSchema(schema.items ?? {})}>`;
  if (schema.type === 'object' && schema.properties) return `object { ${Object.keys(schema.properties).join(', ')} }`;
  return schema.type ? `type: ${schema.type}` : 'schema: n/a';
}

function deref(spec, value) {
  if (!value || typeof value !== 'object' || !value.$ref) return value;
  const path = value.$ref.replace(/^#\//, '').split('/');
  return path.reduce((acc, key) => acc?.[key], spec);
}

function slug(value) {
  return String(value).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}
