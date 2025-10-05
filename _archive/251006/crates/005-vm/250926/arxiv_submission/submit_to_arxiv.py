#!/usr/bin/env python3
"""
arXiv Submission Helper Script
This script helps prepare and submit papers to arXiv.
Since arxiv-cli has compatibility issues, this script provides an alternative approach.
"""

import os
import sys
import json
from pathlib import Path

def check_files():
    """Check if all required files are present."""
    required_files = [
        'paper.pdf',
        'digital_computing_system_paper.md',
        'paper.tex',
        'references.bib',
        'source_code/Cargo.toml',
        'source_code/README.md'
    ]

    missing_files = []
    for file in required_files:
        if not os.path.exists(file):
            missing_files.append(file)

    if missing_files:
        print("❌ Missing required files:")
        for file in missing_files:
            print(f"   - {file}")
        return False

    print("✅ All required files are present")
    return True

def generate_submission_metadata():
    """Generate submission metadata for arXiv."""

    metadata = {
        "title": "Tamaki: An Autopoietic Computing System - EDVAC and Von Neumann Architecture-Based Digital Computing System: A Modern Approach with Data Flow and Small-World Networks",

        "authors": [
            {
                "name": "Jun Kawasaki",
                "affiliation": "Independent Researcher",
                "email": "jun784@junkawasaki.com"
            }
        ],

        "abstract": """This paper presents a modern digital computing system architecture that builds upon the foundational principles of EDVAC (Electronic Discrete Variable Automatic Computer) and Von Neumann architecture, while incorporating contemporary concepts such as data flow execution, heterogeneous computing tiles, and small-world network topologies. The proposed system maintains the sequential execution model of Von Neumann machines as its core, but enhances it with data flow DAG (Directed Acyclic Graph) runtime for task-level parallelism and memoization.

The architecture features a ring-tree (円相) topology with small-world shortcuts, heterogeneous computing tiles (CPU, GPU, CGRA/FPGA, and PIM), and content-addressable caching for redundancy elimination. Through critical path scheduling, NUMA-aware placement, and proximity computing, the system achieves significant performance improvements while maintaining implementation feasibility with current hardware components.

We demonstrate that this approach can reduce average hop counts by 30-70%, eliminate 10-40% of redundant computations through memoization, and provide 2-5x overall performance improvements for general DAG pipelines, with potential for even greater gains in data-intensive workloads.

Validated Results: Our complete Rust prototype implementation demonstrates 5.7x faster DAG scheduling (74.1μs vs 421μs), 35x better sequential memory performance (284ns vs 9.92μs), 78-85% memoization hit rates, and 288x network efficiency improvement at 65k nodes. Large-scale simulations show 35-45% energy savings while delivering 2.3x-4.7x performance improvements across ETL pipelines, ML training, video analytics, and scientific simulation workloads.""",

        "categories": ["cs.AR", "cs.DC", "cs.PF"],

        "keywords": [
            "computer architecture",
            "von neumann architecture",
            "data flow computing",
            "small-world networks",
            "heterogeneous computing",
            "dag scheduling",
            "memoization",
            "rust implementation",
            "performance optimization",
            "distributed systems"
        ],

        "license": "http://arxiv.org/licenses/nonexclusive-distrib/1.0/",

        "comments": "Complete implementation and validation results included. Source code available at: https://github.com/com-junkawasaki/tamaki"
    }

    return metadata

def create_submission_package():
    """Create the final submission package."""

    print("📦 Creating arXiv submission package...")

    # Create a timestamped submission directory
    import datetime
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    submission_dir = f"arxiv_submission_{timestamp}"

    os.makedirs(submission_dir, exist_ok=True)

    # Copy all files to the submission directory
    import shutil

    files_to_copy = [
        'paper.tex',  # MAIN PAPER (TeX source)
        'supplementary_paper.pdf',  # Formatted PDF version
        'digital_computing_system_paper.md',
        'references.bib',
        'README.md'
    ]

    for file in files_to_copy:
        if os.path.exists(file):
            shutil.copy2(file, submission_dir)
            print(f"   ✅ Copied {file}")

    # Copy source code directory
    if os.path.exists('source_code'):
        shutil.copytree('source_code', f'{submission_dir}/source_code')
        print("   ✅ Copied source_code directory")

    # Create submission metadata file
    metadata = generate_submission_metadata()
    with open(f'{submission_dir}/metadata.json', 'w', encoding='utf-8') as f:
        json.dump(metadata, f, indent=2, ensure_ascii=False)
    print("   ✅ Created metadata.json")

    # Create a submission checklist
    checklist = f"""
# arXiv Submission Checklist for: {metadata['title']}

## Files Prepared:
- ✅ paper.tex ({os.path.getsize('paper.tex') / 1024:.1f} KB) - MAIN PAPER (TeX/LaTeX source)
- ✅ supplementary_paper.pdf ({os.path.getsize('paper.pdf') / 1024:.1f} KB) - Formatted PDF version
- ✅ digital_computing_system_paper.md ({os.path.getsize('digital_computing_system_paper.md') / 1024:.1f} KB)
- ✅ references.bib ({os.path.getsize('references.bib') / 1024:.1f} KB)
- ✅ source_code/ (Complete Rust implementation)
- ✅ metadata.json (Submission metadata)
- ✅ README.md (Project overview)

## Submission Metadata:
- **Title**: {metadata['title'][:100]}...
- **Categories**: {', '.join(metadata['categories'])}
- **Authors**: {metadata['authors'][0]['name']} ({metadata['authors'][0]['email']})
- **License**: arXiv Non-Exclusive Distribution License

## Submission Instructions:

### Option 1: Web Interface (Recommended)
1. Go to https://arxiv.org/submit
2. Login with your arXiv account (or create one)
3. Fill in the submission form:
   - Title: Copy from metadata.json
   - Authors: Jun Kawasaki, jun784@junkawasaki.com
   - Abstract: Copy from metadata.json
   - Categories: cs.AR, cs.DC, cs.PF
   - Comments: Include link to source code repository
4. Upload paper.tex as the MAIN PAPER file (TeX/LaTeX source)
5. Upload the entire {submission_dir}/ directory as supplementary material
   - This includes: supplementary_paper.pdf, digital_computing_system_paper.md, source_code/, etc.
6. Review and submit

### IMPORTANT NOTES:
- arXiv requires TeX/LaTeX source (paper.tex) as the main submission
- PDF files generated from TeX are not accepted as main papers
- Supplementary materials can include PDFs, source code, etc.

### Option 2: Direct API (Advanced)
If you have arXiv API access, you can use the metadata.json file for automated submission.

### Option 3: Email Submission (Legacy)
Email the paper.pdf to submit@arxiv.org with subject: "submit <username>"

## Post-Submission:
- Check your submission status at https://arxiv.org/user
- The paper should appear within a few hours
- Share the arXiv URL once published

## Repository:
Full source code and documentation available at:
https://github.com/jun784/junkawasaki-digital-computing-system
"""

    with open(f'{submission_dir}/SUBMISSION_CHECKLIST.txt', 'w') as f:
        f.write(checklist)

    print(f"✅ Created submission package: {submission_dir}")
    print(f"📋 Created submission checklist: {submission_dir}/SUBMISSION_CHECKLIST.txt")

    return submission_dir

def main():
    """Main function to prepare arXiv submission."""
    print("🚀 arXiv Submission Helper for Tamaki Computing System")
    print("=" * 60)

    # Check if we're in the right directory
    if not os.path.exists('paper.pdf'):
        print("❌ Error: paper.pdf not found. Please run this script from the arxiv_submission directory.")
        sys.exit(1)

    # Check required files
    if not check_files():
        print("❌ Please ensure all required files are present.")
        sys.exit(1)

    # Generate submission metadata
    metadata = generate_submission_metadata()
    print("✅ Generated submission metadata")

    # Create submission package
    submission_dir = create_submission_package()

    print("\n" + "=" * 60)
    print("🎉 arXiv Submission Package Ready!")
    print("=" * 60)
    print(f"📁 Submission directory: {submission_dir}")
    print(f"📄 Main paper: paper.tex ({os.path.getsize('paper.tex') / 1024 / 1024:.2f} MB) - TeX/LaTeX source")
    print(f"📦 Supplementary PDF: supplementary_paper.pdf ({os.path.getsize('paper.pdf') / 1024 / 1024:.2f} MB)")
    print(f"📦 Source code: source_code/ ({os.path.getsize('source_code') / 1024 / 1024:.2f} MB)")
    print(f"📋 Checklist: {submission_dir}/SUBMISSION_CHECKLIST.txt")

    print("\n📋 Next Steps:")
    print("1. Review the submission checklist")
    print("2. Go to https://arxiv.org/submit")
    print("3. Follow the instructions in SUBMISSION_CHECKLIST.txt")
    print("4. Submit paper.tex as the MAIN PAPER")
    print("5. Upload the entire directory as supplementary material")

    print(f"\n🔗 Repository: https://github.com/com-junkawasaki/tamaki")

if __name__ == "__main__":
    main()
