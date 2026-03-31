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
});

describe('addAsyncFilter', () => {
  it('registers a filter usable in renderStringAsync', async () => {
    const env = new Environment();
    env.addAsyncFilter('shout', (val) => String(val).toUpperCase());
    const result = await env.renderStringAsync('{{ name | shout }}', { name: 'hello' });
    assert.equal(result, 'HELLO');
  });
});

describe('addAsyncGlobal', () => {
  it('registers a global callable usable in renderStringAsync', async () => {
    const env = new Environment();
    env.addAsyncGlobal('getData', () => 'fetched');
    const result = await env.renderStringAsync('{{ getData() }}', {});
    assert.equal(result, 'fetched');
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
