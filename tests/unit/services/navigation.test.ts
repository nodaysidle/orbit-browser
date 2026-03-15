/**
 * Navigation service tests
 */

import { describe, it, expect } from 'vitest'
import { NavigationService } from '../../../src/main/services/navigation'

describe('NavigationService', () => {
  const service = new NavigationService()
  
  describe('resolveInput', () => {
    it('should handle URLs with protocols', () => {
      const result = service.resolveInput('https://example.com')
      expect(result.url).toBe('https://example.com')
      expect(result.isSearch).toBe(false)
    })
    
    it('should add https to domain-only inputs', () => {
      const result = service.resolveInput('example.com')
      expect(result.url).toBe('https://example.com')
      expect(result.isSearch).toBe(false)
    })
    
    it('should handle search queries with spaces', () => {
      const result = service.resolveInput('hello world')
      expect(result.isSearch).toBe(true)
      expect(result.url).toContain('duckduckgo.com')
    })
    
    it('should encode search queries properly', () => {
      const result = service.resolveInput('hello world')
      expect(result.url).toContain('hello%20world')
    })
    
    it('should handle complex URLs', () => {
      const result = service.resolveInput('https://example.com/path?query=value')
      expect(result.url).toBe('https://example.com/path?query=value')
      expect(result.isSearch).toBe(false)
    })
  })
  
  describe('isValidUrl', () => {
    it('should return true for valid URLs', () => {
      expect(service.isValidUrl('https://example.com')).toBe(true)
      expect(service.isValidUrl('http://localhost:3000')).toBe(true)
    })
    
    it('should return false for invalid URLs', () => {
      expect(service.isValidUrl('not a url')).toBe(false)
      expect(service.isValidUrl('')).toBe(false)
    })
  })
  
  describe('extractDomain', () => {
    it('should extract domain from URL', () => {
      expect(service.extractDomain('https://example.com/path')).toBe('example.com')
    })
    
    it('should handle URLs with subdomains', () => {
      expect(service.extractDomain('https://www.example.com')).toBe('www.example.com')
    })
    
    it('should truncate long invalid inputs', () => {
      const longString = 'a'.repeat(50)
      expect(service.extractDomain(longString).length).toBeLessThanOrEqual(30)
    })
  })
})
