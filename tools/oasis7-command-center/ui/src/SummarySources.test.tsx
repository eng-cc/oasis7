import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import SummarySources from './SummarySources';
import type { DocumentSection } from './types';

afterEach(cleanup);
const section = (path: string, title: string, line = 1): DocumentSection => ({
  id: `${path}#${line}`, path, title, line, content: '旧核验时间与未知部署保持原文',
  revision: 'work-revision', committedRevision: 'source-head', dirty: true,
  factKind: 'sourceFact', lifecycle: null,
});
it('opens exact indexed source versions in source order without including unrelated README files', () => {
  const status = section('doc/engineering/command-center/README.md', '当前判断', 1);
  const coverage = section(status.path, '核验基线与覆盖范围', 20);
  const architecture = section('doc/engineering/command-center/architecture/README.md', '部署边界');
  const showSource = vi.fn();
  render(() => <SummarySources sections={[coverage, section('doc/README.md', '无关入口'), architecture, status]} hasProject showSource={showSource} />);
  const buttons = screen.getAllByRole('button', {hidden:true});
  expect(buttons.map(button => button.textContent)).toEqual(['当前判断', '核验基线与覆盖范围', '部署边界']);
  fireEvent.click(buttons[0]);
  expect(showSource).toHaveBeenCalledWith(status, buttons[0]);
  expect(screen.getByText(/文件读取版本不代表事实已重新核验/)).toBeInTheDocument();
  expect(screen.getAllByText('工作树有修改')).toHaveLength(3);
});
it('keeps the available source navigable when the other summary is outside coverage', () => {
  render(() => <SummarySources sections={[section('doc/engineering/command-center/README.md', '当前判断')]} hasProject showSource={vi.fn()} />);
  expect(screen.getByRole('button', {name: '当前判断'})).toBeEnabled();
  expect(screen.getByText(/architecture\/README.md 未进入当前已读取范围/)).toBeInTheDocument();
});
it('distinguishes no project from missing indexed sources', () => {
  render(() => <SummarySources sections={[]} hasProject={false} showSource={vi.fn()} />);
  expect(screen.getAllByText('打开项目后读取摘要来源。')).toHaveLength(2);
  expect(screen.queryByText(/未进入当前已读取范围/)).not.toBeInTheDocument();
});
