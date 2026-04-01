import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { Environment } from '../index.js';

describe('renderStringAsync', () => {
  it('returns a Promise that resolves to the rendered string', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync('Hello, {{ name }}!', { name: 'World' });
    assert.equal(result, 'Hello, World!');
  });

  it('matches sync renderString output', async () => {
    const env = new Environment();
    const tpl = '{% for x in items %}{{ x }}{% endfor %}';
    const ctx = { items: [1, 2, 3] };
    const sync = env.renderString(tpl, ctx);
    const async_ = await env.renderStringAsync(tpl, ctx);
    assert.equal(async_, sync);
  });

  it('rejects on template errors', async () => {
    const env = new Environment();
    await assert.rejects(
      () => env.renderStringAsync('{{ foo | nonexistentFilter }}', {}),
      (err) => {
        assert.ok(err instanceof Error);
        return true;
      }
    );
  });
});

describe('renderTemplateAsync', () => {
  it('renders a named template from the map', async () => {
    const env = new Environment();
    env.setTemplateMap({ 'hello.html': 'Hi, {{ who }}!' });
    const result = await env.renderTemplateAsync('hello.html', { who: 'async' });
    assert.equal(result, 'Hi, async!');
  });

  it('rejects when template name is missing from map', async () => {
    const env = new Environment();
    env.setTemplateMap({ 'a.njk': 'x' });
    await assert.rejects(() => env.renderTemplateAsync('missing.njk', {}), (err) => {
      assert.ok(err instanceof Error);
      return true;
    });
  });

  it('include via template map works in async render', async () => {
    const env = new Environment();
    env.setTemplateMap({
      'part.njk': '{{ label }}',
      'main.njk': '{% include "part.njk" %}',
    });
    const result = await env.renderTemplateAsync('main.njk', { label: 'ok' });
    assert.equal(result, 'ok');
  });
});

describe('addAsyncFilter', () => {
  it('registers a filter usable in renderStringAsync', async () => {
    const env = new Environment();
    env.addAsyncFilter('shout', (val) => String(val).toUpperCase());
    const result = await env.renderStringAsync('{{ name | shout }}', { name: 'hello' });
    assert.equal(result, 'HELLO');
  });

  it('async filter overrides builtin fast-path (e.g. upper)', async () => {
    const env = new Environment();
    env.addAsyncFilter('upper', (val) => String(val).toUpperCase() + '!');
    const result = await env.renderStringAsync('{{ name | upper }}', { name: 'hello' });
    assert.equal(result, 'HELLO!');
  });

  it('async filter works in filter block tag', async () => {
    const env = new Environment();
    env.addAsyncFilter('shout', (val) => String(val).toUpperCase());
    const result = await env.renderStringAsync('{% filter shout %}hello world{% endfilter %}', {});
    assert.equal(result, 'HELLO WORLD');
  });

  it('async filter with extra argument', async () => {
    const env = new Environment();
    env.addAsyncFilter('rep', (val, n) => String(val).repeat(Number(n) || 1));
    const result = await env.renderStringAsync('{{ "x" | rep(3) }}', {});
    assert.equal(result, 'xxx');
  });
});

describe('addAsyncGlobal', () => {
  it('registers a global callable usable in renderStringAsync', async () => {
    const env = new Environment();
    env.addAsyncGlobal('getData', () => 'fetched');
    const result = await env.renderStringAsync('{{ getData() }}', {});
    assert.equal(result, 'fetched');
  });

  it('passes positional and keyword arguments (Nunjucks-style kwargs object)', async () => {
    const env = new Environment();
    env.addAsyncGlobal('fmt', (a, b, c) => {
      const kwargs = c && typeof c === 'object' && !Array.isArray(c) ? c : {};
      const sep = kwargs.sep ?? ',';
      return `${a}${sep}${b}`;
    });
    const result = await env.renderStringAsync(
      "{{ fmt('a', 'b', sep='|') }}",
      {}
    );
    assert.equal(result, 'a|b');
  });
});

describe('async callback Promise detection', () => {
  it('rejects when addAsyncFilter callback returns a Promise', async () => {
    const env = new Environment();
    env.addAsyncFilter('asyncUpper', async (val) => String(val).toUpperCase());
    await assert.rejects(
      () => env.renderStringAsync('{{ name | asyncUpper }}', { name: 'hello' }),
      (err) => {
        assert.ok(err.message.includes('Promise'));
        return true;
      }
    );
  });

  it('rejects when addAsyncGlobal callback returns a Promise', async () => {
    const env = new Environment();
    env.addAsyncGlobal('fetchData', async () => 'result');
    await assert.rejects(
      () => env.renderStringAsync('{{ fetchData() }}', {}),
      (err) => {
        assert.ok(err.message.includes('Promise'));
        return true;
      }
    );
  });
});

describe('sync render with async-only registrations', () => {
  it('gives clear error when async global used in sync render', () => {
    const env = new Environment();
    env.addAsyncGlobal('getData', () => 'fetched');
    assert.throws(
      () => env.renderString('{{ getData() }}', {}),
      (err) => {
        assert.ok(err.message.includes('async global'));
        assert.ok(err.message.includes('renderStringAsync'));
        return true;
      }
    );
  });

  it('gives clear error when async filter used in sync render', () => {
    const env = new Environment();
    env.addAsyncFilter('shout', (val) => String(val).toUpperCase());
    assert.throws(
      () => env.renderString('{{ name | shout }}', { name: 'hello' }),
      (err) => {
        assert.ok(err.message.includes('async filter'));
        assert.ok(err.message.includes('renderStringAsync'));
        return true;
      }
    );
  });

  it('gives clear error when async filter overrides builtin in sync render', () => {
    const env = new Environment();
    env.addAsyncFilter('upper', (val) => String(val).toUpperCase() + '!');
    assert.throws(
      () => env.renderString('{{ name | upper }}', { name: 'hello' }),
      (err) => {
        assert.ok(err.message.includes('async filter'));
        assert.ok(err.message.includes('renderStringAsync'));
        return true;
      }
    );
  });
});

describe('async template tags', () => {
  it('asyncEach renders in async mode (via sync bridge)', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% asyncEach item in items %}{{ item }}{% endeach %}',
      { items: ['a', 'b', 'c'] }
    );
    assert.equal(result, 'abc');
  });

  it('asyncAll renders in async mode', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% asyncAll item in items %}{{ item }}{% endall %}',
      { items: [1, 2, 3] }
    );
    assert.equal(result, '123');
  });

  it('ifAsync renders in async mode', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% ifAsync show %}visible{% endif %}',
      { show: true }
    );
    assert.equal(result, 'visible');
  });

  it('ifAsync yields empty when condition is false', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% ifAsync show %}visible{% endif %}',
      { show: false }
    );
    assert.equal(result, '');
  });

  it('asyncEach with empty list runs else branch', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% asyncEach x in items %}{{ x }}{% else %}empty{% endeach %}',
      { items: [] }
    );
    assert.equal(result, 'empty');
  });

  it('asyncAll with empty list runs else branch', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% asyncAll x in items %}{{ x }}{% else %}none{% endall %}',
      { items: [] }
    );
    assert.equal(result, 'none');
  });

  it('asyncAll preserves iteration order (sequential engine)', async () => {
    const env = new Environment();
    const result = await env.renderStringAsync(
      '{% asyncAll i in items %}{{ i }}:{% endall %}',
      { items: [3, 1, 4] }
    );
    assert.equal(result, '3:1:4:');
  });

  it('nested asyncEach with async filter', async () => {
    const env = new Environment();
    env.addAsyncFilter('mark', (v) => `[${v}]`);
    const result = await env.renderStringAsync(
      '{% asyncEach row in rows %}{{ row | mark }}{% endeach %}',
      { rows: ['a', 'b'] }
    );
    assert.equal(result, '[a][b]');
  });

  it('sync render rejects asyncEach with clear error', () => {
    const env = new Environment();
    assert.throws(
      () => env.renderString('{% asyncEach x in items %}{{ x }}{% endeach %}', { items: [1] }),
      (err) => {
        assert.ok(err.message.includes('async render mode'));
        return true;
      }
    );
  });
});
