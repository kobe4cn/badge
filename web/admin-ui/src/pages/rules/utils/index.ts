/**
 * 规则画布工具函数导出
 */

export {
  isValidConnection,
  validateConnection,
  getAcceptableSourceTypes,
  getAcceptableTargetTypes,
  type ValidationResult,
} from './connectionValidation';

export {
  canvasToRule,
  ruleToCanvas,
  validateRule,
  serializeRule,
  deserializeRule,
  type RuleCondition,
  type RuleAction,
  type RuleDefinition,
} from './ruleSerializer';
