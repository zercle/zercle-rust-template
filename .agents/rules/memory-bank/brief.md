# Project Brief: Zercle Rust Template

**Initialized:** 2026-01-30  
**Type:** Rust Web API Template  
**Status:** Active Development

## Overview
A production-ready Rust web API template built with Clean Architecture principles. Designed to serve as a foundation for scalable backend services with built-in authentication, database integration, and comprehensive tooling.

## Purpose
- Provide a standardized starting point for Rust web API projects
- Demonstrate Clean Architecture implementation in Rust
- Include essential production features: auth, logging, config management, migrations
- Enable rapid project bootstrap with sensible defaults

## Target Use Cases
- REST API backends for web/mobile applications
- Microservices requiring JWT authentication
- Projects needing PostgreSQL persistence
- Services requiring structured logging and observability

## Core Philosophy
- **Clean Architecture**: Domain-driven design with clear layer separation
- **Production-Ready**: Security, performance, and maintainability built-in
- **Developer Experience**: Clear patterns, comprehensive tooling, easy onboarding
- **Extensibility**: Modular design for easy feature addition

## Current State
- Basic project structure established
- Database connectivity and migrations working
- Configuration system supporting YAML and environment variables
- Health check endpoint operational
- Authentication domain models defined
- Infrastructure layer partially implemented

## Constraints
- PostgreSQL required (no other database support planned)
- JWT-based authentication only
- Axum web framework locked
- Rust 1.75+ minimum version
